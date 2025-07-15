use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::is_local_used;
use clippy_utils::{path_to_local_id, peel_blocks, peel_ref_operators, strip_pat_refs};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Closure, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, UintTy};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for naive byte counts
    ///
    /// ### Why is this bad?
    /// The [`bytecount`](https://crates.io/crates/bytecount)
    /// crate has methods to count your bytes faster, especially for large slices.
    ///
    /// ### Known problems
    /// If you have predominantly small slices, the
    /// `bytecount::count(..)` method may actually be slower. However, if you can
    /// ensure that less than 2³²-1 matches arise, the `naive_count_32(..)` can be
    /// faster in those cases.
    ///
    /// ### Example
    /// ```no_run
    /// # let vec = vec![1_u8];
    /// let count = vec.iter().filter(|x| **x == 0u8).count();
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// # let vec = vec![1_u8];
    /// let count = bytecount::count(&vec, 0u8);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NAIVE_BYTECOUNT,
    pedantic,
    "use of naive `<slice>.filter(|&x| x == y).count()` to count byte values"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    filter_recv: &'tcx Expr<'_>,
    filter_arg: &'tcx Expr<'_>,
) {
    if let ExprKind::Closure(&Closure { body, .. }) = filter_arg.kind
        && let body = cx.tcx.hir_body(body)
        && let [param] = body.params
        && let PatKind::Binding(_, arg_id, _, _) = strip_pat_refs(param.pat).kind
        && let ExprKind::Binary(ref op, l, r) = body.value.kind
        && op.node == BinOpKind::Eq
        && is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(filter_recv).peel_refs(), sym::SliceIter)
        && let operand_is_arg = (|expr| {
            let expr = peel_ref_operators(cx, peel_blocks(expr));
            path_to_local_id(expr, arg_id)
        })
        && let needle = if operand_is_arg(l) {
            r
        } else if operand_is_arg(r) {
            l
        } else {
            return;
        }
        && ty::Uint(UintTy::U8) == *cx.typeck_results().expr_ty(needle).peel_refs().kind()
        && !is_local_used(cx, needle, arg_id)
    {
        let haystack = if let ExprKind::MethodCall(path, receiver, [], _) = filter_recv.kind {
            let p = path.ident.name;
            if p == sym::iter || p == sym::iter_mut {
                receiver
            } else {
                filter_recv
            }
        } else {
            filter_recv
        };
        let mut applicability = Applicability::MaybeIncorrect;
        span_lint_and_sugg(
            cx,
            NAIVE_BYTECOUNT,
            expr.span,
            "you appear to be counting bytes the naive way",
            "consider using the bytecount crate",
            format!(
                "bytecount::count({}, {})",
                snippet_with_applicability(cx, haystack.span, "..", &mut applicability),
                snippet_with_applicability(cx, needle.span, "..", &mut applicability)
            ),
            applicability,
        );
    }
}
