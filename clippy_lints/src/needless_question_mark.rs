use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_lang_ctor;
use clippy_utils::source::snippet;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::LangItem::{OptionSome, ResultOk};
use rustc_hir::{Body, Expr, ExprKind, LangItem, MatchSource, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyS;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Suggests alternatives for useless applications of `?` in terminating expressions
    ///
    /// ### Why is this bad?
    /// There's no reason to use `?` to short-circuit when execution of the body will end there anyway.
    ///
    /// ### Example
    /// ```rust
    /// struct TO {
    ///     magic: Option<usize>,
    /// }
    ///
    /// fn f(to: TO) -> Option<usize> {
    ///     Some(to.magic?)
    /// }
    ///
    /// struct TR {
    ///     magic: Result<usize, bool>,
    /// }
    ///
    /// fn g(tr: Result<TR, bool>) -> Result<usize, bool> {
    ///     tr.and_then(|t| Ok(t.magic?))
    /// }
    ///
    /// ```
    /// Use instead:
    /// ```rust
    /// struct TO {
    ///     magic: Option<usize>,
    /// }
    ///
    /// fn f(to: TO) -> Option<usize> {
    ///    to.magic
    /// }
    ///
    /// struct TR {
    ///     magic: Result<usize, bool>,
    /// }
    ///
    /// fn g(tr: Result<TR, bool>) -> Result<usize, bool> {
    ///     tr.and_then(|t| t.magic)
    /// }
    /// ```
    pub NEEDLESS_QUESTION_MARK,
    complexity,
    "Suggest `value.inner_option` instead of `Some(value.inner_option?)`. The same goes for `Result<T, E>`."
}

declare_lint_pass!(NeedlessQuestionMark => [NEEDLESS_QUESTION_MARK]);

impl LateLintPass<'_> for NeedlessQuestionMark {
    /*
     * The question mark operator is compatible with both Result<T, E> and Option<T>,
     * from Rust 1.13 and 1.22 respectively.
     */

    /*
     * What do we match:
     * Expressions that look like this:
     * Some(option?), Ok(result?)
     *
     * Where do we match:
     *      Last expression of a body
     *      Return statement
     *      A body's value (single line closure)
     *
     * What do we not match:
     *      Implicit calls to `from(..)` on the error value
     */

    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        if let ExprKind::Ret(Some(e)) = expr.kind {
            check(cx, e);
        }
    }

    fn check_body(&mut self, cx: &LateContext<'_>, body: &'_ Body<'_>) {
        check(cx, body.value.peel_blocks());
    }
}

fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    let inner_expr = if_chain! {
        if let ExprKind::Call(path, [arg]) = &expr.kind;
        if let ExprKind::Path(ref qpath) = &path.kind;
        if is_lang_ctor(cx, qpath, OptionSome) || is_lang_ctor(cx, qpath, ResultOk);
        if let ExprKind::Match(inner_expr_with_q, _, MatchSource::TryDesugar) = &arg.kind;
        if let ExprKind::Call(called, [inner_expr]) = &inner_expr_with_q.kind;
        if let ExprKind::Path(QPath::LangItem(LangItem::TryTraitBranch, _)) = &called.kind;
        if expr.span.ctxt() == inner_expr.span.ctxt();
        let expr_ty = cx.typeck_results().expr_ty(expr);
        let inner_ty = cx.typeck_results().expr_ty(inner_expr);
        if TyS::same_type(expr_ty, inner_ty);
        then { inner_expr } else { return; }
    };
    span_lint_and_sugg(
        cx,
        NEEDLESS_QUESTION_MARK,
        expr.span,
        "question mark operator is useless here",
        "try",
        format!("{}", snippet(cx, inner_expr.span, r#""...""#)),
        Applicability::MachineApplicable,
    );
}
