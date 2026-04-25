use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::higher::{If, IfLetOrMatch};
use clippy_utils::msrvs::Msrv;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::source::{IntoSpan, SpanRangeExt, snippet};
use clippy_utils::usage::mutated_variables;
use clippy_utils::visitors::{for_each_expr_without_closures, is_local_used};
use clippy_utils::{SpanlessEq, get_ref_operators, is_unit_expr, peel_blocks_with_stmt, peel_ref_operators};
use core::ops::ControlFlow;
use rustc_ast::BorrowKind;
use rustc_errors::{Applicability, MultiSpan};
use rustc_hir::LangItem::OptionNone;
use rustc_hir::{Arm, Expr, ExprKind, HirId, HirIdSet, Node, Pat, PatExpr, PatExprKind, PatKind};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, Place, PlaceBase, PlaceWithHirId};
use rustc_lint::LateContext;
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty;
use rustc_span::symbol::Ident;
use rustc_span::{BytePos, Span};

use crate::collapsible_if::{parens_around, peel_parens};

use super::{COLLAPSIBLE_MATCH, pat_contains_disallowed_or};

pub(super) fn check_match<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, arms: &'tcx [Arm<'_>], msrv: Msrv) {
    if let Some(els_arm) = arms.iter().rfind(|arm| arm_is_wild_like(cx, arm)) {
        let last_non_wildcard = arms.iter().rposition(|arm| !arm_is_wild_like(cx, arm));
        for (idx, arm) in arms.iter().enumerate() {
            let only_wildcards_after = last_non_wildcard.is_none_or(|lnw| idx >= lnw);
            check_arm(
                cx,
                true,
                arm.pat,
                expr,
                arm.body,
                arm.guard,
                Some(els_arm.body),
                msrv,
                only_wildcards_after,
            );
        }
    }
}

pub(super) fn check_if_let<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    body: &'tcx Expr<'_>,
    else_expr: Option<&'tcx Expr<'_>>,
    let_expr: &'tcx Expr<'_>,
    msrv: Msrv,
) {
    check_arm(cx, false, pat, let_expr, body, None, else_expr, msrv, false);
}

#[expect(clippy::too_many_arguments, clippy::too_many_lines)]
fn check_arm<'tcx>(
    cx: &LateContext<'tcx>,
    outer_is_match: bool,
    outer_pat: &'tcx Pat<'tcx>,
    outer_cond: &'tcx Expr<'tcx>,
    outer_then_body: &'tcx Expr<'tcx>,
    outer_guard: Option<&'tcx Expr<'tcx>>,
    outer_else_body: Option<&'tcx Expr<'tcx>>,
    msrv: Msrv,
    only_wildcards_after: bool,
) {
    let inner_expr = peel_blocks_with_stmt(outer_then_body);
    if let Some(inner) = IfLetOrMatch::parse(cx, inner_expr)
        && let Some((inner_scrutinee, inner_then_pat, inner_else_body)) = match inner {
            IfLetOrMatch::IfLet(scrutinee, pat, _, els, _) => Some((scrutinee, pat, els)),
            IfLetOrMatch::Match(scrutinee, arms, ..) => {
                if arms.len() == 2 && arms.iter().all(|a| a.guard.is_none())
                    // if there are more than two arms, collapsing would be non-trivial
                    // one of the arms must be "wild-like"
                    && let Some(wild_idx) = arms.iter().rposition(|a| arm_is_wild_like(cx, a))
                {
                    let (then, els) = (&arms[1 - wild_idx], &arms[wild_idx]);
                    Some((scrutinee, then.pat, Some(els.body)))
                } else {
                    None
                }
            },
        }
        && outer_pat.span.eq_ctxt(inner_scrutinee.span)
        // match expression must be a local binding
        // match <local> { .. }
        && let Some(binding_id) = peel_ref_operators(cx, inner_scrutinee).res_local_id()
        && !pat_contains_disallowed_or(cx, inner_then_pat, msrv)
        // the binding must come from the pattern of the containing match arm
        // ..<local>.. => match <local> { .. }
        && let (Some((binding_ident, binding_span)), is_innermost_parent_pat_struct) =
            find_pat_binding_and_is_innermost_parent_pat_struct(outer_pat, binding_id)
        // the "else" branches must be equal
        && match (outer_else_body, inner_else_body) {
            (None, None) => true,
            (None, Some(e)) | (Some(e), None) => is_unit_expr(e),
            (Some(a), Some(b)) => SpanlessEq::new(cx).eq_expr(a, b),
        }
        // the binding must not be used in the if guard
        && outer_guard.is_none_or(|e| !is_local_used(cx, e, binding_id))
        // ...or anywhere in the inner expression
        && match inner {
            IfLetOrMatch::IfLet(_, _, body, els, _) => {
                !is_local_used(cx, body, binding_id) && els.is_none_or(|e| !is_local_used(cx, e, binding_id))
            },
            IfLetOrMatch::Match(_, arms, ..) => !arms.iter().any(|arm| is_local_used(cx, arm, binding_id)),
        }
        // Check if the inner expression contains any borrows/dereferences
        && let ref_types = get_ref_operators(cx, inner_scrutinee)
        && let Some(method) = build_ref_method_chain(ref_types)
    {
        let msg = format!(
            "this `{}` can be collapsed into the outer `{}`",
            if matches!(inner, IfLetOrMatch::Match(..)) {
                "match"
            } else {
                "if let"
            },
            if outer_is_match { "match" } else { "if let" },
        );
        // collapsing patterns need an explicit field name in struct pattern matching
        // ex: Struct {x: Some(1)}
        let replace_msg = if is_innermost_parent_pat_struct {
            format!(", prefixed by `{binding_ident}: `")
        } else {
            String::new()
        };
        span_lint_hir_and_then(cx, COLLAPSIBLE_MATCH, inner_expr.hir_id, inner_expr.span, msg, |diag| {
            let mut help_span = MultiSpan::from_spans(vec![binding_span, inner_then_pat.span]);
            help_span.push_span_label(binding_span, "replace this binding");
            help_span.push_span_label(inner_then_pat.span, format!("with this pattern{replace_msg}"));
            if !method.is_empty() {
                let outer_cond_msg = format!("use: `{}{}`", snippet(cx, outer_cond.span, ".."), method);
                help_span.push_span_label(outer_cond.span, outer_cond_msg);
            }
            diag.span_help(
                help_span,
                "the outer pattern can be modified to include the inner pattern",
            );
        });
    } else if outer_is_match // Leave if-let to the `collapsible_if` lint
        && only_wildcards_after // adding a guard allows fall-through; unsafe if other arms follow
        && let Some(inner) = If::hir(inner_expr)
        && outer_pat.span.eq_ctxt(inner.cond.span)
        && match (outer_else_body, inner.r#else) {
            (None, None) => true,
            (None, Some(e)) | (Some(e), None) => is_unit_expr(e),
            (Some(a), Some(b)) => SpanlessEq::new(cx).eq_expr(a, b),
        }
        && !pat_bindings_moved_or_mutated(cx, outer_pat, inner.cond)
        && !inner_cond_conflicts_with_scrutinee_borrows(cx, outer_cond, inner.cond)
    {
        span_lint_hir_and_then(
            cx,
            COLLAPSIBLE_MATCH,
            inner_expr.hir_id,
            inner_expr.span,
            "this `if` can be collapsed into the outer `match`",
            |diag| {
                let outer_then_open_bracket = outer_then_body
                    .span
                    .split_at(1)
                    .0
                    .with_leading_whitespace(cx)
                    .into_span();
                let outer_then_closing_bracket = {
                    let end = outer_then_body.span.shrink_to_hi();
                    end.with_lo(end.lo() - BytePos(1))
                        .with_leading_whitespace(cx)
                        .into_span()
                };
                let outer_arrow_end = if let Some(outer_guard) = outer_guard {
                    outer_guard.span.shrink_to_hi()
                } else {
                    outer_pat.span.shrink_to_hi()
                };
                let (paren_start, inner_if_span, paren_end) = peel_parens(cx, inner_expr.span);
                let inner_if = inner_if_span.split_at(2).0;
                let mut sugg = vec![
                    (inner.then.span.shrink_to_lo(), "=> ".to_string()),
                    (outer_arrow_end.to(outer_then_open_bracket), String::new()),
                    (outer_then_closing_bracket, String::new()),
                ];

                if let Some(outer_guard) = outer_guard {
                    sugg.extend(parens_around(outer_guard));
                    sugg.push((inner_if, "&&".to_string()));
                }

                if !paren_start.is_empty() {
                    sugg.push((paren_start, String::new()));
                }

                if !paren_end.is_empty() {
                    sugg.push((paren_end, String::new()));
                }

                sugg.extend(parens_around(inner.cond));

                if let Some(else_inner) = inner.r#else {
                    let else_inner_span = inner.then.span.shrink_to_hi().to(else_inner.span);
                    sugg.push((else_inner_span, String::new()));
                }

                diag.multipart_suggestion("collapse nested if block", sugg, Applicability::MachineApplicable);
            },
        );
    }
}

/// A "wild-like" arm has a wild (`_`) or `None` pattern and no guard. Such arms can be "collapsed"
/// into a single wild arm without any significant loss in semantics or readability.
fn arm_is_wild_like(cx: &LateContext<'_>, arm: &Arm<'_>) -> bool {
    if arm.guard.is_some() {
        return false;
    }
    match arm.pat.kind {
        PatKind::Binding(..) | PatKind::Wild => true,
        PatKind::Expr(PatExpr {
            kind: PatExprKind::Path(qpath),
            hir_id,
            ..
        }) => cx
            .qpath_res(qpath, *hir_id)
            .ctor_parent(cx)
            .is_lang_item(cx, OptionNone),
        _ => false,
    }
}

fn find_pat_binding_and_is_innermost_parent_pat_struct(pat: &Pat<'_>, hir_id: HirId) -> (Option<(Ident, Span)>, bool) {
    let mut binding = None;
    let mut is_innermost_parent_pat_struct = false;
    pat.walk_short(|p| match p.kind {
        // ignore OR patterns
        PatKind::Or(_) => false,
        PatKind::Binding(_bm, _, ident, _) => {
            let found = p.hir_id == hir_id;
            if found {
                binding = Some((ident, p.span));
            }
            !found
        },
        _ => {
            is_innermost_parent_pat_struct = matches!(p.kind, PatKind::Struct(..));
            true
        },
    });
    (binding, is_innermost_parent_pat_struct)
}

/// Builds a chain of reference-manipulation method calls (e.g., `.as_ref()`, `.as_mut()`,
/// `.copied()`) based on reference operators
fn build_ref_method_chain(expr: Vec<&Expr<'_>>) -> Option<String> {
    let mut req_method_calls = String::new();

    for ref_operator in expr {
        match ref_operator.kind {
            ExprKind::AddrOf(BorrowKind::Raw, _, _) => {
                return None;
            },
            ExprKind::AddrOf(_, m, _) if m.is_mut() => {
                req_method_calls.push_str(".as_mut()");
            },
            ExprKind::AddrOf(_, _, _) => {
                req_method_calls.push_str(".as_ref()");
            },
            // Deref operator is the only operator that this function should have received
            ExprKind::Unary(_, _) => {
                req_method_calls.push_str(".copied()");
            },
            _ => (),
        }
    }

    Some(req_method_calls)
}

/// Checks if any of the bindings in the `pat` are moved or mutated in the `expr`. It is invalid to
/// move or mutate bindings in `if` guards.
fn pat_bindings_moved_or_mutated<'tcx>(cx: &LateContext<'tcx>, pat: &Pat<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let mut delegate = MovedVarDelegate {
        moved: HirIdSet::default(),
    };
    if ExprUseVisitor::for_clippy(cx, expr.hir_id.owner.def_id, &mut delegate)
        .walk_expr(expr)
        .is_err()
    {
        return true;
    }

    let mut candidates = delegate.moved;
    if let Some(mutated) = mutated_variables(expr, cx) {
        candidates.extend(mutated);
    }

    !pat.walk_short(|pat| {
        if let PatKind::Binding(_, hir_id, ..) = pat.kind
            && candidates.contains(&hir_id)
        {
            return false;
        }
        true
    })
}

struct MovedVarDelegate {
    moved: HirIdSet,
}

impl<'tcx> Delegate<'tcx> for MovedVarDelegate {
    fn consume(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        if let PlaceBase::Local(hir_id) = cmt.place.base {
            self.moved.insert(hir_id);
        }
    }

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn borrow(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId, _: ty::BorrowKind) {}
    fn mutate(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}

/// Returns true when collapsing the inner `if cond` of a match arm body into a guard would
/// introduce a borrow conflict: the scrutinee leaves a mutable borrow live on a place that
/// `inner_cond` reads. Guards run while the scrutinee borrow is still active, whereas an arm
/// body may be entered after NLL has already ended the borrow.
fn inner_cond_conflicts_with_scrutinee_borrows<'tcx>(
    cx: &LateContext<'tcx>,
    scrutinee: &'tcx Expr<'tcx>,
    inner_cond: &'tcx Expr<'tcx>,
) -> bool {
    let mut mut_borrows = MutBorrowDelegate::default();
    if ExprUseVisitor::for_clippy(cx, scrutinee.hir_id.owner.def_id, &mut mut_borrows)
        .walk_expr(scrutinee)
        .is_err()
    {
        return false;
    }

    // Expand one level: if the scrutinee references a local of type `&mut T`, follow the
    // borrow back through the binding's let-init. This catches the common let-else pattern
    // `let Some(x) = receiver.method_mut() else { ... };` where `x` carries the borrow into
    // the scrutinee transparently.
    let mut visited = HirIdSet::default();
    let _ = for_each_expr_without_closures(scrutinee, |e| {
        if let Some(local_id) = e.res_local_id()
            && visited.insert(local_id)
            && let ty::Ref(_, _, mutbl) = cx.typeck_results().node_type(local_id).kind()
            && mutbl.is_mut()
            && let Some(init) = let_init_for_binding(cx, local_id)
        {
            let _ = ExprUseVisitor::for_clippy(cx, init.hir_id.owner.def_id, &mut mut_borrows).walk_expr(init);
        }
        ControlFlow::<()>::Continue(())
    });

    if mut_borrows.places.is_empty() {
        return false;
    }

    let mut reads = ReadDelegate::default();
    if ExprUseVisitor::for_clippy(cx, inner_cond.hir_id.owner.def_id, &mut reads)
        .walk_expr(inner_cond)
        .is_err()
    {
        return false;
    }

    mut_borrows
        .places
        .iter()
        .any(|m| reads.places.iter().any(|r| places_alias(m, r)))
}

/// Walks up from a binding `HirId` through enclosing `Pat` nodes to its `LetStmt` and returns
/// the initializer expression, if any. Returns `None` for function parameters or other binding
/// forms.
fn let_init_for_binding<'tcx>(cx: &LateContext<'tcx>, binding_id: HirId) -> Option<&'tcx Expr<'tcx>> {
    let mut id = binding_id;
    loop {
        match cx.tcx.parent_hir_node(id) {
            Node::LetStmt(local) => return local.init,
            Node::Pat(p) => id = p.hir_id,
            _ => return None,
        }
    }
}

/// Returns true if the two places may refer to overlapping memory: same base and one's
/// projection list is a prefix of the other's. `Upvar` bases are treated conservatively.
fn places_alias(a: &Place<'_>, b: &Place<'_>) -> bool {
    match (a.base, b.base) {
        (PlaceBase::Upvar(_), _) | (_, PlaceBase::Upvar(_)) => true,
        (PlaceBase::Local(la), PlaceBase::Local(lb)) if la == lb => {
            let len = a.projections.len().min(b.projections.len());
            a.projections[..len]
                .iter()
                .zip(b.projections[..len].iter())
                .all(|(pa, pb)| pa.kind == pb.kind)
        },
        _ => false,
    }
}

#[derive(Default)]
struct MutBorrowDelegate<'tcx> {
    places: Vec<Place<'tcx>>,
}

impl<'tcx> Delegate<'tcx> for MutBorrowDelegate<'tcx> {
    fn consume(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn borrow(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId, bk: ty::BorrowKind) {
        if matches!(bk, ty::BorrowKind::Mutable) {
            self.places.push(cmt.place.clone());
        }
    }
    fn mutate(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        self.places.push(cmt.place.clone());
    }
    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}

#[derive(Default)]
struct ReadDelegate<'tcx> {
    places: Vec<Place<'tcx>>,
}

impl<'tcx> Delegate<'tcx> for ReadDelegate<'tcx> {
    fn consume(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        self.places.push(cmt.place.clone());
    }
    fn use_cloned(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        self.places.push(cmt.place.clone());
    }
    fn borrow(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId, _: ty::BorrowKind) {
        self.places.push(cmt.place.clone());
    }
    fn mutate(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        self.places.push(cmt.place.clone());
    }
    fn fake_read(&mut self, cmt: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {
        self.places.push(cmt.place.clone());
    }
}
