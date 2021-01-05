use crate::utils::{meets_msrv, snippet_with_applicability, span_lint_and_sugg};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{lint::in_external_macro, ty};
use rustc_semver::RustcVersion;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use std::borrow::Cow;

declare_clippy_lint! {
    /// **What it does:** Checks for uses of `min_value()` and `max_value()` functions of the
    /// primitive integer types.
    ///
    /// **Why is this bad?** Both functions are soft-deprecated with the use of the `MIN` and `MAX`
    /// constants recommended instead.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// let min = i32::min_value();
    /// let max = i32::max_value();
    /// ```
    /// Use instead:
    /// ```rust
    /// let min = i32::MIN;
    /// let max = i32::MAX;
    /// ```
    pub INT_MIN_MAX_VALUE,
    style,
    "use of `min_value()` and `max_value()` for primitive integer types"
}

impl_lint_pass!(IntMinMaxValue => [INT_MIN_MAX_VALUE]);

const INT_MIN_MAX_VALUE_MSRV: RustcVersion = RustcVersion::new(1, 43, 0);

pub struct IntMinMaxValue {
    msrv: Option<RustcVersion>,
}
impl IntMinMaxValue {
    #[must_use]
    pub fn new(msrv: Option<RustcVersion>) -> Self {
        Self { msrv }
    }
}

impl LateLintPass<'_> for IntMinMaxValue {
    extract_msrv_attr!(LateContext);

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if meets_msrv(self.msrv.as_ref(), &INT_MIN_MAX_VALUE_MSRV);
            if !in_external_macro(cx.sess(), expr.span);
            if let ExprKind::Call(func, []) = expr.kind;
            if let ExprKind::Path(QPath::TypeRelative(ty, name)) = func.kind;
            let res_ty = cx.typeck_results().node_type(ty.hir_id);
            if let ty::Int(_) | ty::Uint(_) = res_ty.kind();
            then {
                let (msg, new_name) = if name.ident.as_str() == "max_value" {
                    ("`max_value` is soft-deprecated", "MAX")
                } else if name.ident.as_str() == "min_value" {
                    ("`min_value` is soft-deprecated", "MIN")
                } else {
                    return;
                };


                let mut app = Applicability::MachineApplicable;
                let sugg_path = match snippet_with_applicability(cx, ty.span, "_", &mut app) {
                    // the span for the type includes the method name for some reason, strip it off
                    Cow::Owned(x) => {
                        Cow::Owned(x.rsplitn(2, "::").nth(1).unwrap_or("_").into())
                    }
                    Cow::Borrowed(x) => Cow::Borrowed(x),
                };
                span_lint_and_sugg(cx, INT_MIN_MAX_VALUE, expr.span, msg, "use constant instead", format!("{}::{}", sugg_path, new_name), app);
            }
        }
    }
}
