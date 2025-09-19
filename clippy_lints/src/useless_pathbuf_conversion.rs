use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// **What it does:** Detects unnecessary `&PathBuf::from(...)` when `&Path` would suffice.
    ///
    /// **Why is this bad?** `PathBuf::from` allocates a new heap buffer unnecessarily.
    ///
    /// **Example:**
    /// ```rust
    /// fn use_path(p: &std::path::Path) {}
    /// use_path(&std::path::PathBuf::from("abc"));
    /// ```
    /// Could be written as:
    /// ```rust
    /// fn use_path(p: &std::path::Path) {}
    /// use_path(std::path::Path::new("abc"));
    /// ```
   #[clippy::version = "1.91.0"]
    pub USELESS_PATHBUF_CONVERSION,
    complexity,
    "creating a PathBuf only to take a reference, where Path::new would suffice"
}

declare_lint_pass!(UselessPathbufConversion => [USELESS_PATHBUF_CONVERSION]);

impl<'tcx> LateLintPass<'tcx> for UselessPathbufConversion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Only care about &PathBuf::from(...)
        if let ExprKind::AddrOf(_, _, inner) = &expr.kind
            && let ExprKind::Call(func, args) = &inner.kind
            && let ExprKind::Path(ref qpath) = func.kind
            && let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id()
        {
            // check that the function is `from`
            if cx.tcx.item_name(def_id) != sym::from {
                return;
            }

            // get the type of the function's return value
            let ty = cx.typeck_results().expr_ty(inner);

            if is_type_diagnostic_item(cx, ty, sym::PathBuf)
                && let Some(arg) = args.first()
            {
                let sugg = format!("Path::new({})", snippet(cx, arg.span, ".."));
                span_lint_and_sugg(
                    cx,
                    USELESS_PATHBUF_CONVERSION,
                    expr.span,
                    "unnecessary `PathBuf::from` when a `&Path` is enough",
                    "consider using",
                    sugg,
                    rustc_errors::Applicability::MachineApplicable,
                );
            }
        }
    }
}
