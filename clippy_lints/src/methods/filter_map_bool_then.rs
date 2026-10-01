use super::FILTER_MAP_BOOL_THEN;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::{SpanExt as _, snippet_with_context};
use clippy_utils::ty::is_copy;
use clippy_utils::{CaptureKind, can_move_expr_to_closure, contains_return, is_from_proc_macro, peel_blocks, sym};
use rustc_ast::Mutability;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, HirId, Param, Pat};
use rustc_lint::{LateContext, LintContext as _};
use rustc_middle::ty::Binder;
use rustc_middle::ty::adjustment::Adjust;
use rustc_span::Span;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, arg: &Expr<'_>, call_span: Span) {
    if cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
        && let ExprKind::Closure(closure) = arg.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let value = peel_blocks(body.value)
        // Indexing should be fine as `filter_map` always has 1 input, we unfortunately need both
        // `inputs` and `params` here as we need both the type and the span
        && let param_ty = closure.fn_decl.inputs[0]
        && let param = body.params[0]
        // Issue #11309
        && let param_ty = cx.tcx.liberate_late_bound_regions(
            closure.def_id.to_def_id(),
            Binder::bind_with_vars(
                cx.typeck_results().node_type(param_ty.hir_id),
                cx.tcx.late_bound_vars(cx.tcx.local_def_id_to_hir_id(closure.def_id)),
            ),
        )
        && is_copy(cx, param_ty)
        && let ExprKind::MethodCall(_, recv, [then_arg], _) = value.kind
        && let ExprKind::Closure(then_closure) = then_arg.kind
        && let then_body = peel_blocks(cx.tcx.hir_body(then_closure.body).value)
        && let Some(def_id) = cx.typeck_results().type_dependent_def_id(value.hir_id)
        && cx.tcx.is_diagnostic_item(sym::bool_then, def_id)
        && !expr.span.in_external_macro(cx.sess().source_map())
        && !is_from_proc_macro(cx, expr)
        // Count the number of derefs needed to get to the bool because we need those in the suggestion
        && let needed_derefs = cx.typeck_results().expr_adjustments(recv)
            .iter()
            .filter(|adj| matches!(adj.kind, Adjust::Deref(_)))
            .count()
        && let Some(param_snippet) = param.span.get_text(cx)
        // Splitting the closure is the only thing this lint has to offer, and here it cannot be
        // done at all, not even by hand.
        && !has_conflicting_captures(cx, &param, recv, then_body)
    {
        let mut applicability = Applicability::MachineApplicable;
        let (filter, _) = snippet_with_context(cx, recv.span, expr.span.ctxt(), "..", &mut applicability);
        let (map, _) = snippet_with_context(cx, then_body.span, expr.span.ctxt(), "..", &mut applicability);

        span_lint_and_then(
            cx,
            FILTER_MAP_BOOL_THEN,
            call_span,
            "usage of `bool::then` in `filter_map`",
            |diag| {
                if contains_return(recv) {
                    diag.help("consider using `filter` then `map` instead");
                } else {
                    diag.span_suggestion(
                        call_span,
                        "use `filter` then `map` instead",
                        format!(
                            "filter(|&{param_snippet}| {derefs}{filter}).map(|{param_snippet}| {map})",
                            derefs = "*".repeat(needed_derefs)
                        ),
                        applicability,
                    );
                }
            },
        );
    }
}

/// Returns a set of all bindings found in the given pattern.
fn find_bindings_from_pat(pat: &Pat<'_>) -> FxHashSet<HirId> {
    let mut bindings = FxHashSet::default();
    pat.walk(|p| {
        if let rustc_hir::PatKind::Binding(_, hir_id, _, _) = p.kind {
            bindings.insert(hir_id);
        }
        true
    });
    bindings
}

/// Returns true if splitting the closure would leave the same binding borrowed by both halves,
/// with at least one of them needing it mutably.
///
/// The two closures are alive at the same time once they are arguments to `filter` and `map`, so
/// the borrow checker rejects that. There is no way to write the split by hand either, which is
/// all this lint has to say, so it stays quiet instead.
fn has_conflicting_captures<'tcx>(
    cx: &LateContext<'tcx>,
    param: &Param<'tcx>,
    filter: &'tcx Expr<'tcx>,
    then: &'tcx Expr<'tcx>,
) -> bool {
    let (Some(filter_captures), Some(then_captures)) =
        (can_move_expr_to_closure(cx, filter), can_move_expr_to_closure(cx, then))
    else {
        return false;
    };

    let param_bindings = find_bindings_from_pat(param.pat);
    filter_captures.iter().any(|(hir_id, filter_cap)| {
        !param_bindings.contains(hir_id)
            && then_captures
                .get(hir_id)
                .is_some_and(|then_cap| matches!(*filter_cap | *then_cap, CaptureKind::Ref(Mutability::Mut)))
    })
}
