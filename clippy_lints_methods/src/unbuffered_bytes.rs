use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_trait_method;
use clippy_utils::ty::implements_trait;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `Read::bytes` on types which don't implement `BufRead`.
    ///
    /// ### Why is this bad?
    /// The default implementation calls `read` for each byte, which can be very inefficient for data that’s not in memory, such as `File`.
    ///
    /// ### Example
    /// ```no_run
    /// use std::io::Read;
    /// use std::fs::File;
    /// let file = File::open("./bytes.txt").unwrap();
    /// file.bytes();
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::io::{BufReader, Read};
    /// use std::fs::File;
    /// let file = BufReader::new(File::open("./bytes.txt").unwrap());
    /// file.bytes();
    /// ```
    #[clippy::version = "1.87.0"]
    pub UNBUFFERED_BYTES,
    perf,
    "calling .bytes() is very inefficient when data is not in memory"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) {
    // Lint if the `.bytes()` call is from the `Read` trait and the implementor is not buffered.
    if is_trait_method(cx, expr, sym::IoRead)
        && let Some(buf_read) = cx.tcx.get_diagnostic_item(sym::IoBufRead)
        && let ty = cx.typeck_results().expr_ty_adjusted(recv)
        && !implements_trait(cx, ty, buf_read, &[])
    {
        span_lint_and_help(
            cx,
            UNBUFFERED_BYTES,
            expr.span,
            "calling .bytes() is very inefficient when data is not in memory",
            None,
            "consider using `BufReader`",
        );
    }
}
