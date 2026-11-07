use clippy_utils::diagnostics::span_lint;
use rustc_hir::*;
use rustc_hir_analysis::lower_ty;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    /// Detect the use of floating-point types, definitions, casting and operations.
    ///
    /// ### Why restrict this?
    /// For some embedded systems without a hardware floating-point processor or kernel
    /// development, it can be useful to rule out floating-point numbers, to avoid slowing down
    /// performance or bloating up the program with a soft-float library.
    ///
    /// ### Example
    /// ```no_run
    /// let a = 0.42;
    /// ```
    ///
    /// ```no_run
    /// let float_to_int_cast = a as u32;
    /// ```
    #[clippy::version = "1.101.0"]
    pub FLOATS_USED,
    restriction,
    "use of floating-point integers"
}
declare_lint_pass!(FloatsUsed => [FLOATS_USED]);

fn contains_float(ty: Ty<'_>) -> bool {
    ty.walk().filter_map(|arg| arg.as_type()).any(|t| t.is_floating_point())
}
impl LateLintPass<'_> for FloatsUsed {
    fn check_ty(&mut self, cx: &LateContext<'_>, ty: &rustc_hir::Ty<'_, AmbigArg>) {
        // If we're in a body, return early, so we don't get an ICE from trying to lower the type.
        // check_expr will handle this instead.
        if cx.enclosing_body.is_some() {
            return;
        }

        match ty.kind {
            TyKind::Infer(..) | TyKind::Err(_) => (),
            _ => {
                // Lower the HIR type.
                let lowered_ty = lower_ty(cx.tcx, ty.as_unambig_ty());
                if lowered_ty.is_floating_point() {
                    span_lint(cx, FLOATS_USED, ty.span, "use of floating-point integers");
                }
            },
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let ty = cx.typeck_results().expr_ty(expr);

        // Check if parent expression's type contains floats.
        if let Node::Expr(parent) = cx.tcx.parent_hir_node(expr.hir_id)
            && contains_float(cx.typeck_results().expr_ty(parent))
        {
            // Avoid duplicate lint
            return;
        }

        // Check if the enclosing body's owner has a declared type that contains floats.
        if let Some(body) = cx.enclosing_body {
            let body_owner = cx.tcx.hir_body_owner_def_id(body);
            if contains_float(cx.tcx.type_of(body_owner).instantiate_identity().skip_normalization()) {
                // Avoid duplicate lint
                return;
            }
        }

        if contains_float(ty) {
            span_lint(cx, FLOATS_USED, expr.span, "use of floating-point integers");
        }
    }
}
