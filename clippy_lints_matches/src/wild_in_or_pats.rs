use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{has_non_exhaustive_attr, is_wild};
use rustc_hir::{Arm, Expr, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for wildcard pattern used with others patterns in same match arm.
    ///
    /// ### Why is this bad?
    /// Wildcard pattern already covers any other pattern as it will match anyway.
    /// It makes the code less readable, especially to spot wildcard pattern use in match arm.
    ///
    /// ### Example
    /// ```no_run
    /// # let s = "foo";
    /// match s {
    ///     "a" => {},
    ///     "bar" | _ => {},
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let s = "foo";
    /// match s {
    ///     "a" => {},
    ///     _ => {},
    /// }
    /// ```
    #[clippy::version = "1.42.0"]
    pub WILDCARD_IN_OR_PATTERNS,
    complexity,
    "a wildcard pattern used with others patterns in same match arm"
}

pub(crate) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, arms: &[Arm<'_>]) {
    // first check if we are matching on an enum that has the non_exhaustive attribute
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    if let ty::Adt(adt_def, _) = ty.kind()
        && has_non_exhaustive_attr(cx.tcx, *adt_def)
    {
        return;
    }
    for arm in arms {
        if let PatKind::Or(fields) = arm.pat.kind
            // look for multiple fields in this arm that contains at least one Wild pattern
            && fields.len() > 1 && fields.iter().any(is_wild)
        {
            span_lint_and_help(
                cx,
                WILDCARD_IN_OR_PATTERNS,
                arm.pat.span,
                "wildcard pattern covers any other pattern as it will match anyway",
                None,
                "consider handling `_` separately",
            );
        }
    }
}
