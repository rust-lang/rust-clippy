use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_trait_method;
use clippy_utils::sugg::Sugg;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

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
    pub CLONED_REF_TO_SLICE_REFS,
    perf,
    "cloning a reference for slice references"
}

declare_lint_pass!(ClonedRefToSliceRefs => [CLONED_REF_TO_SLICE_REFS]);

impl<'tcx> LateLintPass<'tcx> for ClonedRefToSliceRefs {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        // `&[foo.clone()]` expressions
        if let ExprKind::AddrOf(_, mutability, arr) = &expr.kind
            // mutable references would have a different meaning
            && mutability.is_not()

            // check for single item arrays
            && let ExprKind::Array([item]) = &arr.kind

            // check for clones
            && let ExprKind::MethodCall(_, val, _, _) = item.kind
            && is_trait_method(cx, item, sym::Clone)

            // get appropriate crate for `slice::from_ref`
            && let Some(builtin_crate) = clippy_utils::std_or_core(cx)
        {
            let mut sugg = Sugg::hir(cx, val, "_");
            if !cx.typeck_results().expr_ty(val).is_ref() {
                sugg = sugg.addr();
            }

            span_lint_and_sugg(
                cx,
                CLONED_REF_TO_SLICE_REFS,
                expr.span,
                format!("this call to `clone` can be replaced with `{builtin_crate}::slice::from_ref`"),
                "try",
                format!("{builtin_crate}::slice::from_ref({sugg})"),
                Applicability::MaybeIncorrect,
            );
        }
    }
}
