use syntax::ast::*;
use rustc::lint::{EarlyLintPass, EarlyContext, LintArray, LintPass};

use utils::*;

declare_lint! {
    pub BLOCK_IN_IF_CONDITION_EXPR, Warn,
    "braces can be eliminated in conditions that are expressions."
}

declare_lint! {
    pub BLOCK_IN_IF_CONDITION_STMT, Warn,
    "avoid complex blocks in conditions, instead the block should be moved higher and bound \
    with 'let'."
}

#[derive(Copy,Clone)]
pub struct BlockInIfCondition;

impl LintPass for BlockInIfCondition {
    fn get_lints(&self) -> LintArray {
        lint_array!(BLOCK_IN_IF_CONDITION_EXPR, BLOCK_IN_IF_CONDITION_STMT)
    }
}

impl EarlyLintPass for BlockInIfCondition {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if let ExprIf(ref check, ref then, _) = expr.node {
            if let ExprBlock(ref block) = check.node {
                if block.stmts.is_empty() {
                    if let Some(ref ex) = block.expr {
                        // zero statements, so remove block

                        span_help_and_lint(cx, BLOCK_IN_IF_CONDITION_EXPR, check.span,
                            "omit braces around single expression condition",
                            &format!("try\nif {} {} ... ", snippet_block(cx, ex.span, ".."),
                            snippet_block(cx, then.span, "..")));
                    }
                } else {
                    // move block higher
                    span_help_and_lint(cx, BLOCK_IN_IF_CONDITION_STMT, check.span,
                        "avoid complex blocks in an 'if' condition; instead, move the block higher \
                        and bind it with a 'let'",
                        &format!("try\nlet res = {};\nif res {} ... ",
                        snippet_block(cx, block.span, ".."),
                        snippet_block(cx, then.span, "..")));
                }
            }
        }
    }
}
