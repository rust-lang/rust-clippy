use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{higher, is_in_test};
use rustc_hir::LangItem::ResultOk;
use rustc_hir::{Expr, LetStmt, Pat, PatKind};
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

fn is_ok_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    if let PatKind::TupleStruct(ref qpath, ..) = pat.kind {
        cx.qpath_res(qpath, pat.hir_id)
            .ctor_parent(cx)
            .is_lang_item(cx, ResultOk)
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
