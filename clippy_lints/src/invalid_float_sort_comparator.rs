use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeResPath as _, MaybeTypeckRes as _};
use clippy_utils::ty::adt_and_variant_of_res;
use clippy_utils::{peel_blocks, sym};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Body, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for floating-point sorting comparators that use `partial_cmp`
    /// with `Ordering::Equal` as a fallback for incomparable values.
    ///
    /// ### Why is this bad?
    /// `partial_cmp` returns `None` when either operand is NaN. Replacing
    /// `None` with `Ordering::Equal` treats NaN as equal to every other value.
    ///
    /// This violates the transitivity required by sorting algorithms. For
    /// example, If 1.0 and NaN compare equal, and NaN and 2.0 compare equal,
    /// then 1.0 and 2.0 must also compare equal. But they don't.
    ///
    /// As a result, sorting may panic or produce incorrectly ordered results.
    ///
    /// ### Example
    /// ```no_run
    /// use std::cmp::Ordering;
    ///
    /// let mut values = vec![1.0_f64, f64::NAN, 2.0];
    /// values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut values = vec![1.0_f64, f64::NAN, 2.0];
    /// values.sort_by(f64::total_cmp);
    /// ```
    ///
    /// Note that `total_cmp` has different semantics: it distinguishes positive
    /// and negative zero and assigns NaNs a defined ordering.
    ///
    /// If the collection is guaranteed not to contain NaNs, use expect instead
    /// to make that assumption explicit:
    /// ```no_run
    /// values.sort_by(|a, b| a.partial_cmp(b).expect("values must not contain NaNs"));
    /// ```
    ///
    /// This preserves the usual floating-point comparison semantics and panics
    /// if the assumption is violated, rather than silently producing an invalid
    /// ordering.
    #[clippy::version = "1.101.0"]
    pub INVALID_FLOAT_SORT_COMPARATOR,
    suspicious,
    "floating-point sorting comparator may violate total ordering"
}

declare_lint_pass!(InvalidFloatSortComparator => [INVALID_FLOAT_SORT_COMPARATOR]);

impl<'tcx> LateLintPass<'tcx> for InvalidFloatSortComparator {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(method, receiver, [comparator], _) = expr.kind
            && matches!(method.ident.name, sym::sort_by | sym::sort_unstable_by)
            && let ty::Slice(element) = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs().kind()
            && element.is_floating_point()
            && cx.ty_based_def(expr).opt_parent(cx).is_impl(cx)
            && let Some(body) = comparator_body(cx, comparator)
            && uses_partial_cmp_with_equal_fallback(cx, body)
        {
            span_lint_and_help(
                cx,
                INVALID_FLOAT_SORT_COMPARATOR,
                comparator.span,
                "this sort comparator treats NaN as equal to every value",
                None,
                "use `total_cmp` for a total order, or `expect` if the values cannot be NaN",
            );
        }
    }
}

fn comparator_body<'tcx>(cx: &LateContext<'tcx>, comparator: &Expr<'tcx>) -> Option<&'tcx Body<'tcx>> {
    match comparator.kind {
        ExprKind::Closure(closure) => Some(cx.tcx.hir_body(closure.body)),
        ExprKind::Path(_) if let Res::Def(DefKind::Fn, id) = comparator.res(cx) => {
            cx.tcx.hir_maybe_body_owned_by(id.as_local()?)
        },
        _ => None,
    }
}

fn uses_partial_cmp_with_equal_fallback(cx: &LateContext<'_>, body: &Body<'_>) -> bool {
    let result = peel_blocks(body.value);
    let typeck = cx.tcx.typeck_body(body.id());

    let ExprKind::MethodCall(unwrap, comparison, [fallback], _) = result.kind else {
        return false;
    };
    let ExprKind::MethodCall(_, lhs, [rhs], _) = comparison.kind else {
        return false;
    };
    if unwrap.ident.name != sym::unwrap_or
        || !typeck
            .ty_based_def(comparison)
            .is_diag_item(cx, sym::cmp_partialord_cmp)
    {
        return false;
    }

    // At a generic function definition the operand types may still be type parameters.
    // Matching the comparator inputs ties them to the float element type at the sort call.
    let [first, second] = body.params else { return false };
    let (Some(lhs), Some(rhs)) = (lhs.res_local_id(), rhs.res_local_id()) else {
        return false;
    };
    let inputs = (first.pat.hir_id, second.pat.hir_id);
    (inputs == (lhs, rhs) || inputs == (rhs, lhs))
        && adt_and_variant_of_res(cx, fallback.res(typeck)).is_some_and(|(_, variant)| variant.name == sym::Equal)
}
