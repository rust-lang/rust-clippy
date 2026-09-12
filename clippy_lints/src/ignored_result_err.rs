use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{higher, is_in_test, peel_blocks};
use rustc_hir::LangItem::{ResultErr, ResultOk};
use rustc_hir::def::Res;
use rustc_hir::{Expr, ExprKind, HirId, LangItem, LetStmt, Pat, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if let Ok(x) = expr`, `while let Ok(x) = expr`, and
    /// `let Ok(x) = expr else { ... }` where the `Err` variant is discarded
    /// without binding.
    ///
    /// ### Why is this bad?
    /// The error value contains context about what went wrong. Discarding it
    /// prevents detailed logging and makes error recovery impossible.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if let Ok(res) = some_call() {
    ///     use_res(res);
    /// } else {
    ///     error!("Something went wrong");
    /// }
    ///
    /// while let Ok(line) = reader.read_line() {
    ///     process(line);
    /// }
    ///
    /// let Ok(val) = some_call() else { return; };
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// match some_call() {
    ///     Ok(res) => use_res(res),
    ///     Err(e) => error!("Something went wrong: {}", e),
    /// }
    ///
    /// loop {
    ///     match reader.read_line() {
    ///         Ok(line) => process(line),
    ///         Err(e) => {
    ///             error!("Read failed: {}", e);
    ///             break;
    ///         }
    ///     }
    /// }
    ///
    /// match some_call() {
    ///     Ok(val) => { /* use val */ },
    ///     Err(e) => {
    ///         error!("Failed: {}", e);
    ///         return;
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.98.0"]
    pub IGNORED_RESULT_ERR,
    restriction,
    "`if let Ok(x) = ...`, `while let Ok(x) = ...`, or `let Ok(x) = ... else` discards the error variant without binding it"
}

impl_lint_pass!(IgnoredResultErr => [IGNORED_RESULT_ERR]);

pub struct IgnoredResultErr {
    allow_in_tests: bool,
}

impl IgnoredResultErr {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            allow_in_tests: conf.allow_ignored_result_err_in_tests,
        }
    }
}

fn is_result_ctor_pattern(cx: &LateContext<'_>, pat: &Pat<'_>, item: LangItem) -> bool {
    if let PatKind::TupleStruct(ref qpath, ..) = pat.kind {
        cx.qpath_res(qpath, pat.hir_id).ctor_parent(cx).is_lang_item(cx, item)
    } else {
        false
    }
}

fn is_ok_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    is_result_ctor_pattern(cx, pat, ResultOk)
}

fn is_err_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    is_result_ctor_pattern(cx, pat, ResultErr)
}

/// If `expr` is a path to a bare local binding (e.g. `sc`), returns its `HirId`.
///
/// Intentionally does not look through field/index projections, so two
/// different places rooted at the same local (`x.a` vs `x.b`) are not treated
/// as equal.
fn as_local(expr: &Expr<'_>) -> Option<HirId> {
    if let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind
        && let Res::Local(hir_id) = path.res
    {
        Some(hir_id)
    } else {
        None
    }
}

/// Returns `true` when the error of an `if let Ok(..) = <local>` is bound in a
/// chained `else if let Err(..) = <same local>` arm.
///
/// In that case the same `Result` value is examined in both arms and the `Err`
/// is bound, so the error is not discarded and the construct should not lint.
/// The scrutinees must be the *same* local binding: two separate calls such as
/// `if let Ok(_) = f() { .. } else if let Err(e) = f() { .. }` are distinct
/// evaluations, and the first call's error really is discarded.
fn err_bound_in_else_chain(cx: &LateContext<'_>, if_let: &higher::IfLet<'_>) -> bool {
    if let Some(else_expr) = if_let.if_else
        && let Some(else_if_let) = higher::IfLet::hir(cx, peel_blocks(else_expr))
        && is_err_pattern(cx, else_if_let.let_pat)
        && let Some(ok_scrutinee) = as_local(if_let.let_expr)
        && let Some(err_scrutinee) = as_local(else_if_let.let_expr)
    {
        ok_scrutinee == err_scrutinee
    } else {
        false
    }
}

impl<'tcx> LateLintPass<'tcx> for IgnoredResultErr {
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'_>) {
        if self.allow_in_tests && is_in_test(cx.tcx, local.hir_id) {
            return;
        }

        if local.els.is_some() && is_ok_pattern(cx, local.pat) {
            span_lint_and_help(
                cx,
                IGNORED_RESULT_ERR,
                local.span,
                "this `let Ok(...) = ... else` discards the `Err` variant",
                None,
                "consider using `match` and binding the `Err` value for logging or recovery",
            );
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if self.allow_in_tests && is_in_test(cx.tcx, expr.hir_id) {
            return;
        }

        if let Some(if_let) = higher::IfLet::hir(cx, expr)
            && is_ok_pattern(cx, if_let.let_pat)
            && !err_bound_in_else_chain(cx, &if_let)
        {
            span_lint_and_help(
                cx,
                IGNORED_RESULT_ERR,
                expr.span,
                "this `if let Ok(...)` discards the `Err` variant",
                None,
                "consider using `match` and binding the `Err` value for logging or recovery",
            );
        } else if let Some(while_let) = higher::WhileLet::hir(expr)
            && is_ok_pattern(cx, while_let.let_pat)
        {
            span_lint_and_help(
                cx,
                IGNORED_RESULT_ERR,
                expr.span,
                "this `while let Ok(...)` discards the `Err` variant",
                None,
                "consider using `loop` + `match` and binding the `Err` value for logging or recovery",
            );
        }
    }
}
