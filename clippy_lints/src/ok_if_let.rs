use rustc::lint::*;
use rustc::hir::*;
use utils::{method_chain_args, span_help_and_lint};
/// **What it does:*** Checks for unnecessary ok() in if let.
///
/// **Why is this bad?** Calling ok() in if let is unnecessary, instead match on Ok()
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rustc
/// for result in iter {
///     if let Some(bench) = try!(result).parse().ok() {
///         vec.push(bench)
///     }
/// }
/// ```
declare_lint! {
    pub OK_IF_LET,
    Warn,
    "usage of ok() in if let statements is unnecessary, match on Ok(expr) instead"
}

#[derive(Copy, Clone)]
pub struct OkIfLetPass;

impl LintPass for OkIfLetPass {
    fn get_lints(&self) -> LintArray {
        lint_array!(OK_IF_LET)
    }
}

impl LateLintPass for OkIfLetPass {
    fn check_expr(&mut self, cx: &LateContext, expr: &Expr) {
        if_let_chain! {[
            let ExprMatch(ref op, ref body, ref source) = expr.node, //test if expr is a match
            let MatchSource::IfLetDesugar { contains_else_clause: _ } = *source, //test if it is an If Let
            let PatKind::TupleStruct(ref x, _, _)  = body[0].pats[0].node, //get operation
            let Some(_) = method_chain_args(op, &["ok"]) //test to see if using ok() method

        ], { 
            if print::path_to_string(x) == "Some" { //if using ok() on a Some, kick in lint
                span_help_and_lint(cx, OK_IF_LET, expr.span,
                "Matching on `Some` with `ok()` is redundant",
                "Consider matching on `Ok()` instead");
            }
        }}
    }
}
