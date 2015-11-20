use syntax::ast::*;
use rustc::lint::{EarlyLintPass, EarlyContext, LintArray, LintPass};

use utils::*;

// TODO: maybe use two lints for this?
declare_lint! {
    pub BLOCK_IN_IF, Warn,
    "braces can be eliminated in conditions that are expressions.  For conditions that have blocks \
     with statements, the block should be moved higher and bound to with 'let'."
}

#[derive(Copy,Clone)]
pub struct BlockInIf;

impl LintPass for BlockInIf {
    fn get_lints(&self) -> LintArray {
        lint_array!(BLOCK_IN_IF)
    }
}

impl EarlyLintPass for BlockInIf {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if let ExprIf(ref check, ref then, _) = expr.node {
            if let ExprBlock(ref block) = check.node {
                if block.stmts.is_empty() {
                    // TODO: its impossible to have an if block with no statements and no
                    // expressions...right?
                    if let Some(ref ex) = block.expr {
                        // zero statements, so remove block

                        // TODO: reprint just condition, or "then" and "else" block as well?

                        span_help_and_lint(cx, BLOCK_IN_IF, check.span,
                            "omit braces around single expression condition",
                            &format!("try\nif {} {} ... ", snippet_block(cx, ex.span, ".."),
                            snippet_block(cx, then.span, "..")));
                    }

                } else {
                    // move block higher
                    span_help_and_lint(cx, BLOCK_IN_IF, check.span,
                        "move block out of condition",
                        &format!("try\nlet res = {};\nif res {} ... ",
                        snippet_block(cx, block.span, ".."),
                        snippet_block(cx, then.span, "..")));
                }
            }
        }
    }
}
