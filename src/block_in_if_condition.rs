use syntax::ast::*;
use rustc::lint::{EarlyLintPass, EarlyContext, LintArray, LintPass};

use utils::*;

declare_lint! {
    pub BLOCK_IN_IF_CONDITION_EXPR, Warn,
    "braces can be eliminated in conditions that are expressions, e.g `if { true } ...`"
}

declare_lint! {
    pub BLOCK_IN_IF_CONDITION_STMT, Warn,
    "avoid complex blocks in conditions, instead move the block higher and bind it \
    with 'let'; e.g: `if { let x = true; x } ...`"
}

#[derive(Copy,Clone)]
pub struct BlockInIfCondition;

impl LintPass for BlockInIfCondition {
    fn get_lints(&self) -> LintArray {
        lint_array!(BLOCK_IN_IF_CONDITION_EXPR, BLOCK_IN_IF_CONDITION_STMT)
    }
}

fn find_bad_block(expr: &Expr_) -> bool {
    match *expr {
        ExprBinary(_, ref left, ref right) => find_bad_block(&left.node) || find_bad_block(&right.node),
        ExprUnary(_, ref exp) => find_bad_block(&exp.node),
        //&ExprBlock(ref block) => is_block_offensive(block), // false positive alert! don't include this
        ExprClosure(_, _, ref block) => {
            if !block.stmts.is_empty() {
                true
            } else {
                if let Some(ref ex) = block.expr {
                    match ex.node {
                        ExprBlock(_) => true,
                        _ => false
                    }
                } else {
                    false
                }
            }
        },
        ExprCall(_, ref args) => args.into_iter().find(|e| find_bad_block(&e.node)).is_some(),
        ExprMethodCall(_, _, ref args) => args.into_iter().find(|e| find_bad_block(&e.node)).is_some(),
        _ => {
            //println!("Dropping out for {:?}", expr);
            false
        },
    }
}

const BRACED_EXPR_MESSAGE:&'static str = "omit braces around single expression condition";
const COMPLEX_BLOCK_MESSAGE:&'static str = "in an 'if' condition, avoid complex blocks or closures with blocks; instead, move the block or closure higher and bind it with a 'let'";

impl EarlyLintPass for BlockInIfCondition {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &Expr) {
        if let ExprIf(ref check, ref then, _) = expr.node {
            if let ExprBlock(ref block) = check.node {
                if block.stmts.is_empty() {
                    if let Some(ref ex) = block.expr {
                        // don't dig into the expression here, just suggest that they remove
                        // the block

                        span_help_and_lint(cx, BLOCK_IN_IF_CONDITION_EXPR, check.span,
                            BRACED_EXPR_MESSAGE,
                            &format!("try\nif {} {} ... ", snippet_block(cx, ex.span, ".."),
                            snippet_block(cx, then.span, "..")));
                    }
                } else {
                    // move block higher
                    span_help_and_lint(cx, BLOCK_IN_IF_CONDITION_STMT, check.span,
                        COMPLEX_BLOCK_MESSAGE,
                        &format!("try\nlet res = {};\nif res {} ... ",
                        snippet_block(cx, block.span, ".."),
                        snippet_block(cx, then.span, "..")));
                }
            } else {
                // go spelunking into the expression, looking for blocks
                if find_bad_block(&check.node) {
                    span_help_and_lint(cx, BLOCK_IN_IF_CONDITION_STMT, check.span,
                        COMPLEX_BLOCK_MESSAGE, "");
                }
            }
        }
    }
}
