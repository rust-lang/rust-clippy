use clippy_utils::consts::{constant, Constant};
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use clippy_utils::{is_direct_expn_of, is_expn_of, match_panic_call};
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `assert!(true)` and `assert!(false)` calls.
    ///
    /// ### Why is this bad?
    /// Will be optimized out by the compiler or should probably be replaced by a
    /// `panic!()` or `unreachable!()`
    ///
    /// ### Known problems
    /// None
    ///
    /// ### Example
    /// ```rust,ignore
    /// assert!(false)
    /// assert!(true)
    /// const B: bool = false;
    /// assert!(B)
    /// ```
    pub ASSERTIONS_ON_CONSTANTS,
    style,
    "`assert!(true)` / `assert!(false)` will be optimized out by the compiler, and should probably be replaced by a `panic!()` or `unreachable!()`"
}

declare_lint_pass!(AssertionsOnConstants => [ASSERTIONS_ON_CONSTANTS]);

impl<'tcx> LateLintPass<'tcx> for AssertionsOnConstants {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        let lint_true = |is_debug: bool| {
            span_lint_and_help(
                cx,
                ASSERTIONS_ON_CONSTANTS,
                e.span,
                if is_debug {
                    "`debug_assert!(true)` will be optimized out by the compiler"
                } else {
                    "`assert!(true)` will be optimized out by the compiler"
                },
                None,
                "remove it",
            );
        };
        let lint_false_without_message = || {
            span_lint_and_help(
                cx,
                ASSERTIONS_ON_CONSTANTS,
                e.span,
                "`assert!(false)` should probably be replaced",
                None,
                "use `panic!()` or `unreachable!()`",
            );
        };
        let lint_false_with_message = |panic_message: String| {
            span_lint_and_help(
                cx,
                ASSERTIONS_ON_CONSTANTS,
                e.span,
                &format!("`assert!(false, {})` should probably be replaced", panic_message),
                None,
                &format!("use `panic!({})` or `unreachable!({})`", panic_message, panic_message),
            );
        };

        if let Some(debug_assert_span) = is_expn_of(e.span, "debug_assert") {
            if debug_assert_span.from_expansion() {
                return;
            }
            if_chain! {
                if let ExprKind::Unary(_, lit) = e.kind;
                if let Some((Constant::Bool(is_true), _)) = constant(cx, cx.typeck_results(), lit);
                if is_true;
                then {
                    lint_true(true);
                }
            };
        } else if let Some(assert_span) = is_direct_expn_of(e.span, "assert") {
            if assert_span.from_expansion() {
                return;
            }
            if let Some(assert_match) = match_assert_with_message(cx, e) {
                match assert_match {
                    // matched assert but not message
                    AssertKind::WithoutMessage(false) => lint_false_without_message(),
                    AssertKind::WithoutMessage(true) | AssertKind::WithMessage(_, true) => lint_true(false),
                    AssertKind::WithMessage(panic_message, false) => lint_false_with_message(panic_message),
                };
            }
        }
    }
}

/// Result of calling `match_assert_with_message`.
enum AssertKind {
    WithMessage(String, bool),
    WithoutMessage(bool),
}

/// Check if the expression matches
///
/// ```rust,ignore
/// if !c {
///   {
///     ::std::rt::begin_panic(message, _)
///   }
/// }
/// ```
///
/// where `message` is any expression and `c` is a constant bool.
fn match_assert_with_message<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> Option<AssertKind> {
    if_chain! {
        if let ExprKind::If(cond, then, _) = expr.kind;
        if let ExprKind::Unary(UnOp::Not, expr) = cond.kind;
        // bind the first argument of the `assert!` macro
        if let Some((Constant::Bool(is_true), _)) = constant(cx, cx.typeck_results(), expr);
        // block
        if let ExprKind::Block(block, _) = then.kind;
        if block.stmts.is_empty();
        if let Some(block_expr) = &block.expr;
        // inner block is optional. unwrap it if it exists, or use the expression as is otherwise.
        if let Some(begin_panic_call) = match block_expr.kind {
            ExprKind::Block(inner_block, _) => &inner_block.expr,
            _ => &block.expr,
        };
        // function call
        if let Some(arg) = match_panic_call(cx, begin_panic_call);
        // bind the second argument of the `assert!` macro if it exists
        if let panic_message = snippet_opt(cx, arg.span);
        // second argument of begin_panic is irrelevant
        // as is the second match arm
        then {
            // an empty message occurs when it was generated by the macro
            // (and not passed by the user)
            return panic_message
                .filter(|msg| !msg.is_empty())
                .map(|msg| AssertKind::WithMessage(msg, is_true))
                .or(Some(AssertKind::WithoutMessage(is_true)));
        }
    }
    None
}
