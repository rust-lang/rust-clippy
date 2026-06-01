use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;

use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::sym::{Error, ToString};
use clippy_utils::ty::implements_trait;
use rustc_hir::{ExprKind, LangItem, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for use of the `Display ` implementation on types that implement `std::error::Error`.
    ///
    /// ### Why is this bad?
    ///
    /// The output emitted from `Display` implementations only includes the error's own description,
    /// without information about any sources that caused it. Code that logs or reports errors
    /// should format them for display using some mechanism that will include information about the
    /// error's sources, such as `std::error::Report`, anyhow`, `eyre`, `display-error-chain`, or
    /// equivalent.
    ///
    /// ### Example
    /// ```rust
    /// use std::ffi::CString;
    ///
    ///fn main() {
    ///    let r = CString::new(vec![0xff]).unwrap().into_string();
    ///    if let Err(e) = r {
    ///        println!("String parsing failed: {e}");
    ///    }
    ///}
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::{error::Error, ffi::CString, fmt::Display};
    ///
    /// // Declare an adapter for Errors that displays source information
    /// struct MyErrorReporter<'a> {
    ///     e: &'a dyn Error,
    /// }
    ///
    /// // Define how to display information about the error and its sources
    /// impl Display for MyErrorReporter<'_> {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "{}", self.e)?;
    ///         let mut maybe_source = self.e.source();
    ///         while let Some(source) = maybe_source {
    ///             write!(f, ": {}", source)?;
    ///             maybe_source = source.source();
    ///         }
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Define a convenient way to construct the adapter
    /// trait AsMyErrorReporter: Error {
    ///     fn as_my_error_reporter<'a>(&'a self) -> MyErrorReporter<'a>;
    /// }
    ///
    /// // Provide a blanket impl that covers all error types
    /// impl<E: Error> AsMyErrorReporter for E {
    ///     fn as_my_error_reporter<'a>(&'a self) -> MyErrorReporter<'a> {
    ///         MyErrorReporter { e: self }
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let r = CString::new(vec![0xff]).unwrap().into_string();
    ///     if let Err(e) = r {
    ///         // Use the blanket method to include source information.
    ///         println!("String parsing failed: {}", e.as_my_error_reporter());
    ///     }
    /// }
    ///
    /// ```
    #[clippy::version = "1.95.0"]
    pub ERROR_FORMAT_WITHOUT_SOURCES,
    suspicious,
    "direct use of an error type's Display implementation"
}

impl_lint_pass!(ErrorFormatWithoutSources => [ERROR_FORMAT_WITHOUT_SOURCES]);

pub struct ErrorFormatWithoutSources {
    msrv: Msrv,
}
impl ErrorFormatWithoutSources {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl<'tcx> LateLintPass<'tcx> for ErrorFormatWithoutSources {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        // If Error.source isn't available yet, let the user modernize more before worrying about this.
        if !self.msrv.meets(cx, msrvs::ERROR_SOURCE) {
            return;
        }
        // Look at function calls
        if let ExprKind::Call(call_path, call_args) = expr.kind
            // Match function calls from the impl of the format_argument struct
            && let ExprKind::Path(QPath::TypeRelative(call_path_hir_ty, call_path_pathsegment)) = call_path.kind
            && let TyKind::Path(QPath::Resolved(_, call_path_hir_ty_path)) = call_path_hir_ty.kind
            && call_path_hir_ty_path.res.is_lang_item(cx, LangItem::FormatArgument)
            // specifically the two associated with the Display or Debug traits
            && let display_sym = Symbol::intern("new_display")
            && let debug_sym = Symbol::intern("new_debug")
            && let call_fnname = call_path_pathsegment.ident.name
            && (call_fnname == display_sym || call_fnname == debug_sym)
            // and the argument is an error type that could have a source error
            && call_args.len() == 1
            && let call_arg_expr = call_args[0]
            && let call_arg_ty = cx.typeck_results().expr_ty_adjusted(&call_arg_expr)
            && ty_impls_error_with_source(cx, call_arg_ty)
        {
            // Identify the type of format trait, based on the function name
            let trait_name = if call_fnname == debug_sym {
                "Debug"
            } else if call_fnname == display_sym {
                "Display"
            } else {
                // This should never happen
                return;
            };

            format_lint(cx, call_arg_expr.span, trait_name, call_arg_ty);
        }

        // Also match method calls
        if let ExprKind::MethodCall(path, receiver, [], to_string_span) = expr.kind
            // If the method receiver is an error that potentially has source information
            && let receiver_ty = cx.typeck_results().expr_ty_adjusted(receiver)
            && ty_impls_error_with_source(cx, receiver_ty)
            // and the method being called is to_string
            && path.ident.name == sym::to_string
            // from the ToString trait
            && cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, ToString)
        {
            format_lint(cx, to_string_span, "Display", receiver_ty);
        }
    }
}

/// Check for an error type that could have a linked source error.
///
/// This function checks if the given type implements `std::error::Error`, and provides a
/// non-default implementation of the `source` method. Formatting operations on an error that has no
/// sources won't discard useful information, so there's no need to emit a diagnostic for those
/// types.
fn ty_impls_error_with_source<'tcx>(cx: &LateContext<'tcx>, some_ty: Ty<'tcx>) -> bool {
    // If the type implements the Error trait
    if let Some(error_def_id) = cx.tcx.get_diagnostic_item(Error)
    && implements_trait(cx, some_ty, error_def_id, &[])
    // we know it's an error now
    && let some_error_ty = some_ty.peel_refs()
    // and the type defines a source method
    && cx
        .tcx
        .non_blanket_impls_for_ty(error_def_id, some_error_ty)
        .flat_map(|impl_id| {
            cx.tcx
                .associated_items(impl_id)
                .filter_by_name_unhygienic(Symbol::intern("source"))
        })
        .next()
        .is_some()
    {
        true
    } else {
        false
    }
}

fn format_lint<'tcx>(cx: &LateContext<'tcx>, usage_span: Span, trait_name: &str, error_ty: Ty<'tcx>) {
    let error_ty = error_ty.peel_refs();
    span_lint_and_help(
        cx,
        ERROR_FORMAT_WITHOUT_SOURCES,
        usage_span,
        format!("use of `{trait_name}` formatting on an error type (`{error_ty}`)",),
        None,
        format!(
            "The '{error_ty}' type supports providing cause information. Directly invoking a string formatting operation on it will discard any information provided by source errors. Instead, use an error reporter that will recursively call `Error::source()` and include available information from source errors in the formatted output."
        ),
    );
}
