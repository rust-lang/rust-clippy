use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::match_def_path;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `BufReader::new` with `Stdin` or `StdinLock`.
    ///
    /// ### Why is this bad?
    /// `Stdin` is already buffered. Re-buffering it only increase the memcpy calls.
    ///
    /// ### Example
    /// ```no_run
    /// let reader = std::io::BufReader::new(std::io::stdin());
    /// ```
    /// Use instead:
    /// ```no_run
    /// let stdin = std::io::stdin();
    /// let reader = stdin.lock();
    /// ```
    #[clippy::version = "1.84.0"]
    pub BUFREADER_STDIN,
    perf,
    "using `BufReader::new` with `Stdin` or `StdinLock` is unnecessary and less efficient"
}

declare_lint_pass!(BufreaderStdinlock => [BUFREADER_STDIN]);

impl<'tcx> LateLintPass<'tcx> for BufreaderStdinlock {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &rustc_hir::Expr<'tcx>) {
        if e.span.from_expansion() {
            return;
        }
        if let ExprKind::Call(func, [arg]) = e.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, seg)) = func.kind
            && seg.ident.name == sym::new
            && let TyKind::Path(ref qpath) = ty.kind
            && let Some(did) = cx.qpath_res(qpath, ty.hir_id).opt_def_id()
            && match_def_path(cx, did, &["std", "io", "buffered", "bufreader", "BufReader"])
            && let arg_ty = cx.typeck_results().expr_ty(arg)
            && let Some(arg_did) = arg_ty.ty_adt_def().map(rustc_middle::ty::AdtDef::did)
        {
            if match_def_path(cx, arg_did, &["std", "io", "stdio", "Stdin"]) {
                span_lint_and_sugg(
                    cx,
                    BUFREADER_STDIN,
                    e.span,
                    "using `BufReader::new` with `Stdin`",
                    "instead of wrapping `Stdin` in `BufReader`, use the `lock` method directly",
                    format!("{}.lock()", snippet(cx, arg.span, "..")),
                    Applicability::MachineApplicable,
                );
            } else if match_def_path(cx, arg_did, &["std", "io", "stdio", "StdinLock"]) {
                span_lint_and_sugg(
                    cx,
                    BUFREADER_STDIN,
                    e.span,
                    "using `BufReader::new` with `Stdin`",
                    "instead of wrapping `StdinLock` in `BufReader`, use it self",
                    snippet(cx, arg.span, "..").to_string(),
                    Applicability::MachineApplicable,
                );
            } else {
                return;
            };
        }
    }
}
