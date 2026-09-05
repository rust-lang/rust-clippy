use crate::methods::method_call;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `Iterator::map` surrounded by calls to `Iterator::rev`,
    /// such as `iter.rev().map(...).rev()`.
    ///
    /// ### Why is this bad?
    /// The two reversals cancel each other out. In particular, the mapping function
    /// runs in the iterator's original order, not in reverse order. This can make the
    /// code's behavior misleading.
    ///
    /// ### Example
    /// ```no_run
    /// let mut total = 0;
    /// let sums: Vec<_> = [1, 2, 3]
    ///     .into_iter()
    ///     .rev()
    ///     .map(|value| {
    ///         total += value;
    ///         total
    ///     })
    ///     .rev()
    ///     .collect();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut total = 0;
    /// let sums: Vec<_> = [1, 2, 3]
    ///     .into_iter()
    ///     .map(|value| {
    ///         total += value;
    ///         total
    ///     })
    ///     .collect();
    /// ```
    ///
    /// If the mapping function should run in reverse order while the results remain in
    /// the original order, collect the reversed iterator and then reverse the collection:
    /// ```no_run
    /// let mut total = 0;
    /// let mut sums: Vec<_> = [1, 2, 3]
    ///     .into_iter()
    ///     .rev()
    ///     .map(|value| {
    ///         total += value;
    ///         total
    ///     })
    ///     .collect();
    /// sums.reverse();
    /// ```
    #[clippy::version = "1.100.0"]
    pub DOUBLE_REV_MAP,
    suspicious,
    "calling `Iterator::rev` before and after `Iterator::map`"
}

declare_lint_pass!(DoubleRevMap => [DOUBLE_REV_MAP]);

impl LateLintPass<'_> for DoubleRevMap {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        let is_iterator_method = |call: &Expr<'_>| {
            cx.ty_based_def(call)
                .assoc_fn_parent(cx)
                .is_diag_item(cx, sym::Iterator)
        };

        if !expr.span.from_expansion()
            && let Some((sym::rev, map_call, [], _, _)) = method_call(expr)
            && let Some((sym::map, inner_rev_call, [_map_args], _, _)) = method_call(map_call)
            && let Some((sym::rev, iter, [], _, _)) = method_call(inner_rev_call)
            && is_iterator_method(expr)
            && is_iterator_method(map_call)
            && is_iterator_method(inner_rev_call)
        {
            let inner_rev_span = inner_rev_call.span.with_lo(iter.span.hi());
            let outer_rev_span = expr.span.with_lo(map_call.span.hi());

            span_lint_and_then(
                cx,
                DOUBLE_REV_MAP,
                expr.span,
                "the calls to `rev` cancel each other, so `map` runs in the original order",
                |diag| {
                    diag.help(
                        "if `map` should run in reverse order, collect after the first `rev` and then reverse the collection",
                    );
                    diag.multipart_suggestion(
                        "remove both calls to `rev`",
                        vec![(inner_rev_span, String::new()), (outer_rev_span, String::new())],
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }
}
