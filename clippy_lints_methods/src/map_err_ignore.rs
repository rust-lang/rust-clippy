use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_hir::{CaptureBy, Closure, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for instances of `map_err(|_| Some::Enum)`
    ///
    /// ### Why restrict this?
    /// This `map_err` throws away the original error rather than allowing the enum to
    /// contain and report the cause of the error.
    ///
    /// ### Example
    /// Before:
    /// ```no_run
    /// use std::fmt;
    ///
    /// #[derive(Debug)]
    /// enum Error {
    ///     Indivisible,
    ///     Remainder(u8),
    /// }
    ///
    /// impl fmt::Display for Error {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         match self {
    ///             Error::Indivisible => write!(f, "could not divide input by three"),
    ///             Error::Remainder(remainder) => write!(
    ///                 f,
    ///                 "input is not divisible by three, remainder = {}",
    ///                 remainder
    ///             ),
    ///         }
    ///     }
    /// }
    ///
    /// impl std::error::Error for Error {}
    ///
    /// fn divisible_by_3(input: &str) -> Result<(), Error> {
    ///     input
    ///         .parse::<i32>()
    ///         .map_err(|_| Error::Indivisible)
    ///         .map(|v| v % 3)
    ///         .and_then(|remainder| {
    ///             if remainder == 0 {
    ///                 Ok(())
    ///             } else {
    ///                 Err(Error::Remainder(remainder as u8))
    ///             }
    ///         })
    /// }
    /// ```
    ///
    /// After:
    /// ```rust
    /// use std::{fmt, num::ParseIntError};
    ///
    /// #[derive(Debug)]
    /// enum Error {
    ///     Indivisible(ParseIntError),
    ///     Remainder(u8),
    /// }
    ///
    /// impl fmt::Display for Error {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         match self {
    ///             Error::Indivisible(_) => write!(f, "could not divide input by three"),
    ///             Error::Remainder(remainder) => write!(
    ///                 f,
    ///                 "input is not divisible by three, remainder = {}",
    ///                 remainder
    ///             ),
    ///         }
    ///     }
    /// }
    ///
    /// impl std::error::Error for Error {
    ///     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    ///         match self {
    ///             Error::Indivisible(source) => Some(source),
    ///             _ => None,
    ///         }
    ///     }
    /// }
    ///
    /// fn divisible_by_3(input: &str) -> Result<(), Error> {
    ///     input
    ///         .parse::<i32>()
    ///         .map_err(Error::Indivisible)
    ///         .map(|v| v % 3)
    ///         .and_then(|remainder| {
    ///             if remainder == 0 {
    ///                 Ok(())
    ///             } else {
    ///                 Err(Error::Remainder(remainder as u8))
    ///             }
    ///         })
    /// }
    /// ```
    #[clippy::version = "1.48.0"]
    pub MAP_ERR_IGNORE,
    restriction,
    "`map_err` should not ignore the original error"
}

pub(super) fn check(cx: &LateContext<'_>, e: &Expr<'_>, arg: &Expr<'_>) {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(e.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
        && is_type_diagnostic_item(cx, cx.tcx.type_of(impl_id).instantiate_identity(), sym::Result)
        && let ExprKind::Closure(&Closure {
            capture_clause: CaptureBy::Ref,
            body,
            fn_decl_span,
            ..
        }) = arg.kind
        && let closure_body = cx.tcx.hir_body(body)
        && let [param] = closure_body.params
        && let PatKind::Wild = param.pat.kind
    {
        // span the area of the closure capture and warn that the
        // original error will be thrown away
        #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
        span_lint_and_then(
            cx,
            MAP_ERR_IGNORE,
            fn_decl_span,
            "`map_err(|_|...` wildcard pattern discards the original error",
            |diag| {
                diag.help(
                    "consider storing the original error as a source in the new error, or silence this warning using an ignored identifier (`.map_err(|_foo| ...`)",
                );
            },
        );
    }
}
