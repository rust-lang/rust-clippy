use super::ERR_EXPECT;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::ty::implements_trait;
use clippy_utils::{meets_msrv, msrvs, ty::is_type_diagnostic_item};
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_middle::ty::Ty;
use rustc_semver::RustcVersion;
use rustc_span::{sym, Span};

pub(super) fn check(
    cx: &LateContext<'_>,
    _expr: &rustc_hir::Expr<'_>,
    recv: &rustc_hir::Expr<'_>,
    msrv: Option<RustcVersion>,
    expect_span: Span,
    err_span: Span,
) {
    if_chain! {
        if is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Result);
        // Test the version to make sure the lint can be showed (expect_err has been
        // introduced in rust 1.17.0 : https://github.com/rust-lang/rust/pull/38982)
        if meets_msrv(msrv, msrvs::EXPECT_ERR);

        // Grabs the `Result<T, E>` type
        let result_type = cx.typeck_results().expr_ty(recv);
        // Tests if the T type in a `Result<T, E>` is not None
        if let Some(data_type) = get_data_type(cx, result_type);
        // Tests if the T type in a `Result<T, E>` implements debug
        if has_debug_impl(data_type, cx);

        then {
            span_lint_and_sugg(
                cx,
                ERR_EXPECT,
                err_span.to(expect_span),
                "called `.err().expect()` on a `Result` value",
                "try",
                "expect_err".to_string(),
                Applicability::MachineApplicable
        );
        }
    };
}

/// Given a `Result<T, E>` type, return its data (`T`).
fn get_data_type<'a>(cx: &LateContext<'_>, ty: Ty<'a>) -> Option<Ty<'a>> {
    match ty.kind() {
        ty::Adt(_, substs) if is_type_diagnostic_item(cx, ty, sym::Result) => substs.types().next(),
        _ => None,
    }
}

/// Given a type, very if the Debug trait has been impl'd
fn has_debug_impl<'tcx>(ty: Ty<'tcx>, cx: &LateContext<'tcx>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::Debug)
        .map_or(false, |debug| implements_trait(cx, ty, debug, &[]))
}
