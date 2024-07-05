use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Body, Closure, Expr, ExprKind, LangItem, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for `Iterator::map` over ranges without using the parameter which
    /// could be more clearly expressed using `std::iter::repeat_with(...).take(...)`.
    ///
    /// ### Why is this bad?
    ///
    /// It expresses the intent more clearly to `take` the correct number of times
    /// from a generating function than to apply a closure to each number in a
    /// range only to discard them.
    ///
    /// ### Example
    /// ```no_run
    /// let random_numbers : Vec<_> = (0..10).map(|_| { 3 + 1 }).collect();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let f : Vec<_> = std::iter::repeat_with(|| { 3 + 1 }).take(10).collect();
    /// ```
    #[clippy::version = "1.81.0"]
    pub MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
    style,
    "map of a trivial closure (not dependent on parameter) over a range"
}

declare_lint_pass!(MapWithUnusedArgumentOverRanges => [MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES]);

impl LateLintPass<'_> for MapWithUnusedArgumentOverRanges {
    fn check_expr(&mut self, cx: &LateContext<'_>, ex: &Expr<'_>) {
        if let ExprKind::MethodCall(path, receiver, [map_arg_expr], _call_span) = ex.kind
            && path.ident.name == rustc_span::sym::map
            && let ExprKind::Struct(qpath, fields, _) = receiver.kind
            && matches!(qpath, QPath::LangItem(LangItem::Range, _))
            && fields.len() == 2
            && let ExprKind::Closure(Closure { body, .. }) = map_arg_expr.kind
            && let Body { params: [param], .. } = cx.tcx.hir().body(*body)
            && matches!(param.pat.kind, PatKind::Wild)
            && let ExprKind::Lit(lit) = fields[0].expr.kind
            && let LitKind::Int(Pu128(lower_bound), _) = lit.node
        {
            if let ExprKind::Lit(lit) = fields[1].expr.kind
                && let LitKind::Int(Pu128(upper_bound), _) = lit.node
            {
                let count = upper_bound - lower_bound;
                let mut applicability = Applicability::MaybeIncorrect;
                let snippet = snippet_with_applicability(cx, map_arg_expr.span, "|| { ... }", &mut applicability)
                    .replacen("|_|", "||", 1);
                span_lint_and_sugg(
                    cx,
                    MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
                    ex.span,
                    "map of a trivial closure (not dependent on parameter) over a range",
                    "use",
                    format!("std::iter::repeat_with({snippet}).take({count})"),
                    applicability,
                );
            } else if lower_bound == 0 {
                let mut applicability = Applicability::MaybeIncorrect;
                let count = snippet_with_applicability(cx, fields[1].expr.span, "...", &mut applicability);
                let snippet = snippet_with_applicability(cx, map_arg_expr.span, "|| { ... }", &mut applicability)
                    .replacen("|_|", "||", 1);
                span_lint_and_sugg(
                    cx,
                    MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
                    ex.span,
                    "map of a trivial closure (not dependent on parameter) over a range",
                    "use",
                    format!("std::iter::repeat_with({snippet}).take({count})"),
                    applicability,
                );
            }
        }
    }
}
