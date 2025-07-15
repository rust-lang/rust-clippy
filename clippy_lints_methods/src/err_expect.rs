use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::ty::{has_debug_impl, is_type_diagnostic_item};
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `.err().expect()` calls on the `Result` type.
    ///
    /// ### Why is this bad?
    /// `.expect_err()` can be called directly to avoid the extra type conversion from `err()`.
    ///
    /// ### Example
    /// ```should_panic
    /// let x: Result<u32, &str> = Ok(10);
    /// x.err().expect("Testing err().expect()");
    /// ```
    /// Use instead:
    /// ```should_panic
    /// let x: Result<u32, &str> = Ok(10);
    /// x.expect_err("Testing expect_err");
    /// ```
    #[clippy::version = "1.62.0"]
    pub ERR_EXPECT,
    style,
    r#"using `.err().expect("")` when `.expect_err("")` can be used"#
}

pub(super) fn check(
    cx: &LateContext<'_>,
    _expr: &rustc_hir::Expr<'_>,
    recv: &rustc_hir::Expr<'_>,
    expect_span: Span,
    err_span: Span,
    msrv: Msrv,
) {
    if is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Result)
        // Grabs the `Result<T, E>` type
        && let result_type = cx.typeck_results().expr_ty(recv)
        // Tests if the T type in a `Result<T, E>` is not None
        && let Some(data_type) = get_data_type(cx, result_type)
        // Tests if the T type in a `Result<T, E>` implements debug
        && has_debug_impl(cx, data_type)
        && msrv.meets(cx, msrvs::EXPECT_ERR)
    {
        span_lint_and_sugg(
            cx,
            ERR_EXPECT,
            err_span.to(expect_span),
            "called `.err().expect()` on a `Result` value",
            "try",
            "expect_err".to_string(),
            Applicability::MachineApplicable,
        );
    }
}

/// Given a `Result<T, E>` type, return its data (`T`).
fn get_data_type<'a>(cx: &LateContext<'_>, ty: Ty<'a>) -> Option<Ty<'a>> {
    match ty.kind() {
        ty::Adt(_, args) if is_type_diagnostic_item(cx, ty, sym::Result) => args.types().next(),
        _ => None,
    }
}
