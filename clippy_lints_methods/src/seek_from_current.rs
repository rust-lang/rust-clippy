use rustc_ast::ast::{LitIntType, LitKind};
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
use clippy_utils::{is_enum_variant_ctor, sym};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks if the `seek` method of the `Seek` trait is called with `SeekFrom::Current(0)`,
    /// and if it is, suggests using `stream_position` instead.
    ///
    /// ### Why is this bad?
    ///
    /// Readability. Use dedicated method.
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// use std::fs::File;
    /// use std::io::{self, Write, Seek, SeekFrom};
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut f = File::create("foo.txt")?;
    ///     f.write_all(b"Hello")?;
    ///     eprintln!("Written {} bytes", f.seek(SeekFrom::Current(0))?);
    ///
    ///     Ok(())
    /// }
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// use std::fs::File;
    /// use std::io::{self, Write, Seek, SeekFrom};
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut f = File::create("foo.txt")?;
    ///     f.write_all(b"Hello")?;
    ///     eprintln!("Written {} bytes", f.stream_position()?);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.67.0"]
    pub SEEK_FROM_CURRENT,
    complexity,
    "use dedicated method for seek from current position"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>, arg: &'tcx Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(recv);

    if let Some(def_id) = cx.tcx.get_diagnostic_item(sym::IoSeek)
        && implements_trait(cx, ty, def_id, &[])
        && arg_is_seek_from_current(cx, arg)
    {
        let mut applicability = Applicability::MachineApplicable;
        let snip = snippet_with_applicability(cx, recv.span, "..", &mut applicability);

        span_lint_and_sugg(
            cx,
            SEEK_FROM_CURRENT,
            expr.span,
            "using `SeekFrom::Current` to start from current position",
            "replace with",
            format!("{snip}.stream_position()"),
            applicability,
        );
    }
}

fn arg_is_seek_from_current<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    if let ExprKind::Call(f, [arg]) = expr.kind
        && let ExprKind::Path(ref path) = f.kind
        && let Some(ctor_call_id) = cx.qpath_res(path, f.hir_id).opt_def_id()
        && is_enum_variant_ctor(cx, sym::SeekFrom, sym::Current, ctor_call_id)
        // check if argument of `SeekFrom::Current` is `0`
        && let ExprKind::Lit(lit) = arg.kind
        && let LitKind::Int(Pu128(0), LitIntType::Unsuffixed) = lit.node
    {
        return true;
    }

    false
}
