use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sugg::Sugg;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for slice references with cloned references such as `&[f.clone()]`.
    ///
    /// ### Why is this bad
    ///
    /// A reference does not need to be owned in order to used as a slice.
    ///
    /// ### Known problems
    ///
    /// This lint does not know whether or not a clone has side effects such as with `Rc` clones or clones with interior
    /// mutability.
    ///
    /// ### Example
    ///
    /// ```ignore
    /// let data = 10;
    /// let data_ref = &data;
    /// take_slice(&[data_ref.clone()]);
    /// ```
    /// Use instead:
    /// ```ignore
    /// use std::slice;
    /// let data = 10;
    /// let data_ref = &data;
    /// take_slice(slice::from_ref(data_ref));
    /// ```
    #[clippy::version = "1.87.0"]
    pub CLONED_REFS_TO_SLICE_REFS,
    perf,
    "cloning a reference for slice references"
}

declare_lint_pass!(ClonedRefsToSliceRefs => [CLONED_REFS_TO_SLICE_REFS]);

impl<'tcx> LateLintPass<'tcx> for ClonedRefsToSliceRefs {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, exp: &Expr<'tcx>) {
        // `&[foo.clone()]` expressions
        if let ExprKind::AddrOf(_, mutability, arr) = &exp.kind &&
            // mutable references would have a different meaning
            mutability.is_not() &&

            // check for single item arrays
            let ExprKind::Array([item]) = &arr.kind &&

            // check for clones
            let ExprKind::MethodCall(call, val, _, _) = &item.kind &&
            call.ident.name.as_str() == "clone" &&

            // get appropriate crate for `slice::from_ref`
            let Some(builtin_crate) = clippy_utils::std_or_core(cx)
        {
            let mut sugg = Sugg::hir(cx, val, "<item>");
            if !cx.typeck_results().expr_ty(val).is_ref() {
                sugg = sugg.addr();
            }

            span_lint_and_sugg(
                cx,
                CLONED_REFS_TO_SLICE_REFS,
                exp.span,
                "this call to clone can be replaced with `std::slice::from_ref`",
                "try",
                format!("{builtin_crate}::slice::from_ref({sugg})"),
                Applicability::MachineApplicable,
            );
        }
    }
}
