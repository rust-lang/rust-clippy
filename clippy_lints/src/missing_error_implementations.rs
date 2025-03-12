use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::ty::implements_trait;
use clippy_utils::{is_from_proc_macro, is_no_core_crate, is_no_std_crate};
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::Visibility;
use rustc_session::impl_lint_pass;
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
    /// ```ignore
    /// #[derive(Debug)]
    /// pub struct ParseError;
    ///
    /// impl core::fmt::Display for ParseError { ... }
    /// ```
    /// Use instead:
    /// ```ignore
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

pub struct MissingErrorImplementations {
    msrv: Msrv,
}

impl MissingErrorImplementations {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl_lint_pass!(MissingErrorImplementations => [MISSING_ERROR_IMPLEMENTATIONS]);

impl<'tcx> LateLintPass<'tcx> for MissingErrorImplementations {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let error_in_core = self.msrv.meets(cx, msrvs::ERROR_IN_CORE);
        if is_no_core_crate(cx) || (is_no_std_crate(cx) && !error_in_core) {
            return;
        }
        match item.kind {
            ItemKind::Enum(_, generics) | ItemKind::Struct(_, generics) => {
                if
                // Only check public items, as missing impls for private items can be fixed when needed without
                // changing publich API.
                (cx.tcx.visibility(item.owner_id.def_id) == Visibility::Public)
                    // Identify types which likely represent errors
                    && {
                        let name: &str = item.ident.name.as_str();
                        name.ends_with("Error") && name != "Error"
                    }
                    // Skip types not controlled by the user
                    && !item.span.in_external_macro(cx.sess().source_map())
                    && !is_from_proc_macro(cx, item)
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
                        format!(
                            "error type doesn't implement `{}::error::Error`",
                            if error_in_core { "core" } else { "std" }
                        ),
                    );
                }
            },
            _ => {},
        }
    }
}
