use clippy_utils::{
    diagnostics::span_lint_and_sugg, meets_msrv, msrvs, source::snippet_with_context, ty::is_type_lang_item,
};
use rustc_errors::Applicability;
use rustc_hir::{lang_items, BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass};
use rustc_semver::RustcVersion;
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Finds arrays or vectors being converted to slices of the same length.
    /// ### Why is this bad?
    /// The methods `as_slice()` or `as_mut_slice()` could be used instead.
    /// ### Example
    /// ```rust
    /// let mut arr: [u32; 3] = [1, 2, 3];
    /// let arr_slice = &arr[..];
    /// let mutable_arr_slice = &mut arr[..];
    ///
    /// let mut vec = vec![1, 2, 3];
    /// let vec_slice = &vec[..];
    /// let mutable_vec_slice = &mut vec[..];
    /// ```
    /// Use instead:
    /// ```rust
    /// let mut arr: [u32; 3] = [1, 2, 3];
    /// let arr_slice = arr.as_slice();
    /// let mutable_arr_slice = arr.as_mut_slice();
    ///
    /// let mut vec = vec![1, 2, 3];
    /// let vec_slice = vec.as_slice();
    /// let mutable_vec_slice = vec.as_mut_slice();
    /// ```
    #[clippy::version = "1.60.0"]
    pub MANUAL_SLICE,
    restriction,
    "Suggest use of array and Vec .as_slice() and .as_mut_slice() methods"
}

#[derive(Clone)]
pub struct ManualSlice {
    msrv: Option<RustcVersion>,
}

impl ManualSlice {
    #[must_use]
    pub fn new(msrv: Option<RustcVersion>) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(ManualSlice => [MANUAL_SLICE]);

impl<'tcx> LateLintPass<'tcx> for ManualSlice {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !meets_msrv(self.msrv.as_ref(), &msrvs::NON_EXHAUSTIVE) {
            return;
        }
        let ctxt = expr.span.ctxt();
        if_chain! {
            if let ExprKind::AddrOf(BorrowKind::Ref, mutability, inner) = &expr.kind;
            if let ExprKind::Index(object, range) = &inner.kind;
            if is_type_lang_item(cx, cx.typeck_results().expr_ty_adjusted(range), lang_items::LangItem::RangeFull);
            then {
                let mut app = Applicability::MachineApplicable;
                let snip = snippet_with_context(cx, object.span, ctxt, "..", &mut app).0;
                let suggested_method = match mutability {
                    Mutability::Not => "to_slice()",
                    Mutability::Mut => "to_mut_slice()",
                };
                span_lint_and_sugg(
                    cx,
                    MANUAL_SLICE,
                    expr.span,
                    "converting to a slice of the same length",
                    "use",
                    format!("{}.{}", snip, suggested_method),
                    app
                );
            }
        }
    }

    extract_msrv_attr!(LateContext);
}
