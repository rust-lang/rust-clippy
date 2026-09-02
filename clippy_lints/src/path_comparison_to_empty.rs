use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for comparing a `Path` or `PathBuf` to `Path::new("")` or `PathBuf::new()` and suggests to use `.is_empty()` instead
    /// ### Why is this bad?
    /// using `is_empty()` is clearer
    /// ### Example
    /// ```no_run
    /// # use std::path::{Path};
    /// let path = Path::new("123");
    /// let _ = path == Path::new("");
    /// ```
    /// Use instead:
    /// ```no_run
    ///  # use std::path::{Path};
    /// let path = Path::new("123");
    /// let _ = path.is_empty();
    /// ```
    #[clippy::version = "1.100.0"]
    pub PATH_COMPARISON_TO_EMPTY,
    style,
    "checking comparison `path == Path::new(\"\")` when `.is_empty()` could be used instead"
}

impl_lint_pass!(PathComparisonToEmpty => [PATH_COMPARISON_TO_EMPTY]);

pub struct PathComparisonToEmpty {
    msrv: Msrv,
}

impl PathComparisonToEmpty {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl LateLintPass<'_> for PathComparisonToEmpty {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        if let ExprKind::Binary(op, left, right) = expr.kind
            && (op.node == BinOpKind::Eq || op.node == BinOpKind::Ne)
            && self.msrv.meets(cx, msrvs::PATH_IS_EMPTY)
        {
            let left_path_new_empty_string = is_path_new_empty_args(cx, left);
            let right_path_new_empty_string = is_path_new_empty_args(cx, right);

            // in case of `Path::new("") == Path::new("")` the lint doesn't make much sense
            if !(left_path_new_empty_string ^ right_path_new_empty_string) {
                return;
            }

            let cmp_expr = if right_path_new_empty_string { left } else { right };
            let cmp_ty = cx.typeck_results().expr_ty(cmp_expr).peel_refs();

            if !matches!(cmp_ty.opt_diag_name(cx), Some(sym::Path | sym::PathBuf)) {
                return;
            }

            let is_non_equal = op.node == BinOpKind::Ne;
            let negation_sign = if is_non_equal { "!" } else { "" };
            let mut applicability = Applicability::MachineApplicable;

            span_lint_and_sugg(
                cx,
                PATH_COMPARISON_TO_EMPTY,
                expr.span,
                "comparing a path against an empty path",
                format!("using `{negation_sign}is_empty` is clearer and more explicit"),
                format!(
                    "{negation_sign}{}.is_empty()",
                    snippet_with_context(cx, cmp_expr.span, expr.span.ctxt(), "_", &mut applicability).0,
                ),
                applicability,
            );
        }
    }
}

fn is_path_new_empty_args(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::Call(func, args) = expr.kind
        && let ExprKind::Path(ref qpath) = func.kind
        && let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id()
        && cx.tcx.item_name(def_id) == sym::new
        && let Some(impl_ty) = def_id.assoc_parent(cx).opt_impl_ty(cx)
    {
        // `PathBuf::new()` takes no args, `Path::new("")` takes one empty string literal.
        return match impl_ty.skip_binder().opt_diag_name(cx) {
            Some(sym::PathBuf) => args.is_empty(),
            Some(sym::Path) => matches!(args, [arg] if is_empty_string(arg)),
            _ => false,
        };
    }
    false
}

fn is_empty_string(expr: &Expr<'_>) -> bool {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Str(lit, _) = lit.node
    {
        let lit = lit.as_str();
        return lit.is_empty();
    }
    false
}
