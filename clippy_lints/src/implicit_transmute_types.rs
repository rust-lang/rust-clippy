use clippy_utils::{diagnostics::span_lint_and_then, is_diagnostic_item_or_ctor, last_path_segment};
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to [`transmute`] without explicit type parameters
    /// (i.e. without turbofish syntax).
    ///
    /// ### Why is this bad?
    /// In most cases Rust's type inference is helpful, however it can cause
    /// problems with [`transmute`]. `transmute` is wildly unsafe
    /// unless the types being transmuted are known to be compatible. As such,
    /// a seemingly innocent change in something's type can end up making a
    /// previously-valid transmute suddenly become unsound. Thus it is
    /// good practice to always be explicit about the types you expect to be
    /// transmuting between, so that the compiler will force you to
    /// reexamine the transmute if either type changes.
    ///
    /// ### Example
    /// ```rust
    /// #[repr(transparent)]
    /// struct CharWrapper {
    ///     _inner: char,
    /// }
    ///
    /// let wrapped = CharWrapper { _inner: 'a' };
    /// let transmuted = unsafe { core::mem::transmute(wrapped) };
    ///
    /// // This is sound now, but if it gets changed in the future to
    /// // something that expects a type other than `char`, the transmute
    /// // would infer it returns that type, which is likely unsound.
    /// let _ = char::is_lowercase(transmuted);
    /// ```
    ///
    /// Specify type parameters:
    /// ```rust
    /// # #[repr(transparent)]
    /// # struct CharWrapper {
    /// #     _inner: char,
    /// # }
    /// let wrapped = CharWrapper { _inner: 'a' };
    /// // Because we explicitly specify the types for the transmute, any change in
    /// // surrounding code that would cause the transmute call to infer different
    /// // types will now be caught by typechecking, forcing us to come back and
    /// // reassess the soundsness of transmuting between the new types.
    /// let transmuted = unsafe { core::mem::transmute::<CharWrapper, char>(wrapped) };
    ///
    /// let _ = char::is_lowercase(transmuted);
    /// ```
    ///
    /// If you decide that you *do* want the types to be inferred,
    /// you can silence the lint by conveying your intention explicitly:
    /// ```rust
    /// # use std::mem::transmute;
    /// # fn main() {
    /// #     unsafe {
    /// #         let foo: i32 = 123;
    /// #         let _: u32 =
    /// transmute::<_, _>(foo);
    /// #     }
    /// # }
    /// ```
    ///
    /// [`transmute`]: https://doc.rust-lang.org/core/mem/fn.transmute.html
    #[clippy::version = "1.72.0"]
    pub IMPLICIT_TRANSMUTE_TYPES,
    style,
    "calling mem::transmute without explicit type parameters"
}

declare_lint_pass!(ImplicitTransmuteTypes => [IMPLICIT_TRANSMUTE_TYPES]);

impl<'tcx> LateLintPass<'tcx> for ImplicitTransmuteTypes {
    fn check_expr(&mut self, cx: &LateContext<'_>, e: &hir::Expr<'_>) {
        if let ExprKind::Call(func, _) = &e.kind
            && let ExprKind::Path(qpath) = &func.kind
            && let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id()
            && is_diagnostic_item_or_ctor(cx, def_id, sym::transmute)
            && last_path_segment(qpath).args.is_none()
        {
            let suggestion_span = qpath.span().shrink_to_hi();

            let substs = cx.typeck_results().node_substs(func.hir_id);
            let srctype = substs.type_at(0);
            let dsttype = substs.type_at(1);


            span_lint_and_then(
                cx,
                IMPLICIT_TRANSMUTE_TYPES,
                e.span,
                "`transmute` called without explicit type parameters",
                |b| {
                    b.span_suggestion_verbose(
                        suggestion_span,
                        "consider specifying the types intended to be transmuted",
                        format!("::<{srctype}, {dsttype}>"),
                        Applicability::MachineApplicable
                    );
                }
            );
        }
    }
}
