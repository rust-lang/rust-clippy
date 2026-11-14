use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::VecArgs;
use clippy_utils::res::{MaybeDef as _, MaybeResPath as _};
use clippy_utils::{eq_expr_value, is_integer_literal, local_is_initialized, peel_ref_operators, sym};
use rustc_attr_ir::lang_items::LangItem;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind, HirId, Node, QPath, TyKind};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, PlaceBase, PlaceWithHirId};
use rustc_lint::LateContext;
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty::BorrowKind;
use rustc_span::Symbol;

use super::NEW_INSTEAD_OF_CLEAR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    if !expr.span.from_expansion() {
        check_collection_new(cx, expr, lhs, rhs);
        check_vec_macro(cx, expr, lhs, rhs);
    }
}

fn check_collection_new(cx: &LateContext<'_>, expr: &Expr<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    if !rhs.span.from_expansion()
        && let ExprKind::Call(func, []) = rhs.kind
        && let ExprKind::Path(QPath::TypeRelative(ty, method)) = func.kind
        && method.ident.name == sym::new
        // the right-hand side may be the only thing fixing the collection's type parameters,
        // through explicit type arguments (`Vec::<u64>::new()`) or a type alias (`VU64::new()`),
        // in which case replacing it with `.clear()` would change type inference
        && names_struct_without_args(ty)
        && let rhs_ty = cx.typeck_results().node_type(ty.hir_id)
        // `.clear()` only keeps the backing allocation for contiguously allocated types;
        // node-based collections (`BTreeMap`, `BTreeSet`, `LinkedList`) don't preallocate,
        // so the note about retained memory must not be shown for them
        && let Some(retains_allocation) = match rhs_ty.opt_diag_name(cx) {
            Some(sym::Vec | sym::HashMap | sym::HashSet | sym::VecDeque | sym::BinaryHeap) => Some(true),
            Some(sym::BTreeMap | sym::BTreeSet | sym::LinkedList) => Some(false),
            _ if rhs_ty.is_lang_item(cx, LangItem::String) => Some(true),
            _ => None,
        }
        && let Some(name) = local_name(cx, expr, lhs)
    {
        emit_lint(cx, expr, name, retains_allocation, None);
    }
}

fn check_vec_macro(cx: &LateContext<'_>, expr: &Expr<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    // `VecArgs::hir` already verifies that the `vec!` macro is used
    let discarded_init = match VecArgs::hir(cx, rhs) {
        Some(VecArgs::Vec([])) => None,
        Some(VecArgs::Repeat(init, count)) if is_integer_literal(count, 0) => Some(init),
        _ => return,
    };
    if let Some(name) = local_name(cx, expr, lhs) {
        // an empty `vec![]` is a `Vec`, which keeps its allocation on `.clear()`
        emit_lint(cx, expr, name, true, discarded_init);
    }
}

fn emit_lint(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    name: Symbol,
    retains_allocation: bool,
    discarded_init: Option<&Expr<'_>>,
) {
    span_lint_and_then(
        cx,
        NEW_INSTEAD_OF_CLEAR,
        expr.span,
        "assigning a new empty collection",
        |diag| {
            diag.span_suggestion(
                expr.span,
                "consider using `.clear()` instead",
                format!("{name}.clear()"),
                Applicability::MaybeIncorrect,
            );
            if retains_allocation {
                diag.note("`.clear()` retains the allocated memory for reuse");
            }
            // for `vec![init; 0]`, `init` is evaluated once but would be skipped by `.clear()`
            if let Some(init) = discarded_init
                && !eq_expr_value(cx, expr.span.ctxt(), init, init)
            {
                diag.span_note(init.span, "side effects of this expression will no longer be executed");
            }
        },
    );
}

/// Checks whether `ty` is a path naming a struct directly, without generic arguments,
/// e.g. `Vec` in `Vec::new()`, but not `Vec::<u64>` or a type alias such as `VU64`.
fn names_struct_without_args(ty: &rustc_hir::Ty<'_>) -> bool {
    matches!(ty.kind, TyKind::Path(QPath::Resolved(None, path))
             if matches!(path.res, Res::Def(DefKind::Struct, _))
                 && path.segments.iter().all(|seg| seg.args.is_none()))
}

/// If `expr` is a path to an initialized local that `assign` doesn't re-initialize after a
/// move, returns the local's name.
///
/// Reference operators (`&`/`*` on references) are peeled, but smart-pointer `Deref`s
/// such as `Box` are deliberately left in place: a `*boxed = Vec::new()` must not be
/// rewritten to `boxed.clear()`, since the smart pointer may have its own `clear` with
/// different semantics.
fn local_name(cx: &LateContext<'_>, assign: &Expr<'_>, expr: &Expr<'_>) -> Option<Symbol> {
    peel_ref_operators(cx, expr)
        .res_local_id_and_ident()
        .filter(|(hir_id, _)| local_is_initialized(cx, *hir_id) && !may_be_moved_at(cx, *hir_id, assign))
        .map(|(_, ident)| ident.name)
}

/// Checks whether the local may have been moved out of when `assign` runs. The assignment
/// then re-initializes the local, and `.clear()` would be a use after move.
///
/// A move counts if it comes before the assignment, or if it is inside a loop that also
/// contains the assignment but not the local's declaration, since the next iteration then
/// reaches the assignment after the move.
fn may_be_moved_at(cx: &LateContext<'_>, local_id: HirId, assign: &Expr<'_>) -> bool {
    let Some(body_id) = cx.enclosing_body else {
        return true;
    };
    let mut delegate = MoveDelegate {
        local_id,
        moves: Vec::new(),
    };
    ExprUseVisitor::for_clippy(cx, cx.tcx.hir_body_owner_def_id(body_id), &mut delegate)
        .consume_body(cx.tcx.hir_body(body_id))
        .into_ok();
    if delegate.moves.is_empty() {
        return false;
    }

    let decl_span = cx.tcx.hir_span(local_id);
    let outer_loop = cx
        .tcx
        .hir_parent_iter(assign.hir_id)
        .filter_map(|(_, node)| match node {
            Node::Expr(e) if matches!(e.kind, ExprKind::Loop(..)) && !e.span.contains(decl_span) => Some(e.span),
            _ => None,
        })
        .last();

    delegate
        .moves
        .iter()
        .map(|&id| cx.tcx.hir_span(id))
        .any(|span| span.lo() < assign.span.lo() || outer_loop.is_some_and(|l| l.contains(span)))
}

/// Collects the expressions that move out of a local.
struct MoveDelegate {
    local_id: HirId,
    moves: Vec<HirId>,
}

impl<'tcx> Delegate<'tcx> for MoveDelegate {
    fn consume(&mut self, place_with_id: &PlaceWithHirId<'tcx>, diag_expr_id: HirId) {
        if place_with_id.place.base == PlaceBase::Local(self.local_id) {
            self.moves.push(diag_expr_id);
        }
    }

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn borrow(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId, _: BorrowKind) {}

    fn mutate(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}
