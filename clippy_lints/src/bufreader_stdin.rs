use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `BufReader::new` with `Stdin` or `StdinLock`.
    ///
    /// ### Why is this bad?
    /// `Stdin` is already buffered. Re-buffering it only increase the memcpy calls.
    ///
    /// ### Example
    ///
    /// ```ignore
    /// let reader = std::io::BufReader::new(std::io::stdin());
    /// ```
    /// Use instead:
    /// ```ignore
    /// let stdin = std::io::stdin();
    /// let reader = stdin.lock();
    /// ```

    #[clippy::version = "1.97.0"]
    pub BUFREADER_STDIN,
    perf,
    "using `BufReader::new` with `Stdin` or `StdinLock` is unnecessary and less efficient"
}

impl_lint_pass!(BufreaderStdin => [BUFREADER_STDIN]);

pub struct BufreaderStdin {}

impl BufreaderStdin {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'tcx> LateLintPass<'tcx> for BufreaderStdin {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Call(func, [arg]) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
            && segment.ident.name == sym::new
            && let TyKind::Path(ref qpath) = ty.kind
            && let Some(did) = cx.qpath_res(qpath, ty.hir_id).opt_def_id()
            && cx.tcx.def_path_str(did).ends_with("std::io::BufReader")
            && let arg_ty = cx.typeck_results().expr_ty(arg)
            && let Some(arg_did) = arg_ty.ty_adt_def().map(rustc_middle::ty::AdtDef::did)
        {
            if cx.tcx.is_diagnostic_item(sym::Stdin, arg_did) {
                span_lint_and_sugg(
                    cx,
                    BUFREADER_STDIN,
                    expr.span,
                    "using `BufReader::new` with `Stdin`",
                    "instead of wrapping `Stdin` in `BufReader`, use the `lock` method directly",
                    format!("{}.lock()", snippet(cx, arg.span, "..")),
                    Applicability::MachineApplicable,
                );
            } else if cx.tcx.def_path_str(arg_did).ends_with("std::io::StdinLock") {
                span_lint_and_sugg(
                    cx,
                    BUFREADER_STDIN,
                    expr.span,
                    "using `BufReader::new` with `StdinLock`",
                    "instead of wrapping `StdinLock` in `BufReader`, use it directly",
                    snippet(cx, arg.span, "..").to_string(),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
