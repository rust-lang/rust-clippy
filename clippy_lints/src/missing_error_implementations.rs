use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::implements_trait;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Visibility;
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for potentially forgotten implementations of `Error` for public types.
    ///
    /// ### Why is this bad?
    /// `Error` provides a common interface for errors.
    /// Errors not implementing `Error` can not be used with functions that expect it.
    ///
    /// ### Example
    /// ```no_run
    /// #[derive(Debug)]
    /// pub struct ParseError;
    ///
    /// impl core::fmt::Display for ParseError { ... }
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[derive(Debug)]
    /// pub struct ParseError;
    ///
    /// impl core::fmt::Display for ParseError { ... }
    ///
    /// impl core::error::Error for ParseError { ... }
    /// ```
    #[clippy::version = "1.87.0"]
    pub MISSING_ERROR_IMPLEMENTATIONS,
    suspicious,
    "exported types with potentially forgotten `Error` implementation"
}

declare_lint_pass!(MissingErrorImplementations => [MISSING_ERROR_IMPLEMENTATIONS]);

impl<'tcx> LateLintPass<'tcx> for MissingErrorImplementations {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        match item.kind {
            ItemKind::Enum(_, generics) | ItemKind::Struct(_, generics) => {
                let is_error_candidate = {
                    let name: &str = item.ident.name.as_str();
                    name.ends_with("Error") && name != "Error"
                };
                if is_error_candidate
                    // Only check public items, as missing impls for private items are easy to fix.
                    && (cx.tcx.visibility(item.owner_id.def_id) == Visibility::Public)
                    // Check whether Error is implemented,
                    // skipping generic types as we'd have to ask whether there is an error impl
                    // for any instantiation of it.
                    && generics.params.is_empty()
                    && let ty = cx.tcx.type_of(item.owner_id).instantiate_identity()
                    && let Some(error_def_id) = cx.tcx.get_diagnostic_item(sym::Error)
                    && !implements_trait(cx, ty, error_def_id, &[])
                {
                    span_lint(
                        cx,
                        MISSING_ERROR_IMPLEMENTATIONS,
                        item.ident.span,
                        "error type doesn't implement `Error`",
                    );
                }
            },
            _ => {},
        }
    }
}
