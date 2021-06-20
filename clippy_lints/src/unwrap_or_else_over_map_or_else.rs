use clippy_utils::diagnostics::span_lint_and_help;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Suggest the use of Result::unwrap_or_else over Result::map_or_else if map_or_else is just used to unpack a successful result while handling an error
    ///
    /// **Why is this bad?** The unwrap_or_else is shorter and more descriptive
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// func_result(in_num: u8) -> Result<&'static str, &'static str> {
    ///    if in_num % 2 != 0 {
    ///        return Err("Can't do this because input is odd...");
    ///    }
    ///    Ok("An even number :)")
    /// }
    ///
    /// func_result(2).map_or_else(|e| println!("{:?}", e), |n| println!("{}", n))
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    ///   let c = func_result(3).unwrap_or_else(|e| {e});
    ///
    /// ```
    pub UNWRAP_OR_ELSE_OVER_MAP_OR_ELSE,
    pedantic,
    "use 'Result::unwrap_or_else' over 'Result::map_or_else'"
}

declare_lint_pass!(UnwrapOrElseOverMapOrElse => [UNWRAP_OR_ELSE_OVER_MAP_OR_ELSE]);

impl LateLintPass<'_> for UnwrapOrElseOverMapOrElse {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if_chain! {
            //check if this is a method call eg func.map_or_else()
            if let ExprKind::MethodCall(method, t_span, args, _) = expr.kind;
            //check if the function name is map_or_else
            if method.ident.as_str() == "map_or_else";
            //check if the first arg is a closure
            if let ExprKind::Closure(_, _, body_id, _, _) = args[1].kind ;
            //get closure body parameter
            let closure_body = cx.tcx.hir().body(body_id);
            //make sure it has a parameter of one
            if closure_body.params.len() == 1;
            then{
                span_lint_and_help(
                    cx,
                    UNWRAP_OR_ELSE_OVER_MAP_OR_ELSE,
                    t_span,
                    "Result::unwrap_or_else is shorter and more succinet",
                    None,
                    "consider unwrap_or_else(|e| handle_the_error(e)) to unpack result",
                );
            }
        }
    }
}
