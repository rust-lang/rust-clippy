use crate::utils::span_lint;
use if_chain::if_chain;
use rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use rustc::{declare_lint_pass, declare_tool_lint};
use syntax::ast;
use syntax::parse::{parser, token};

declare_clippy_lint! {
    /// **What it does:** Checks for missing parameters in `panic!`.
    ///
    /// **Why is this bad?** Contrary to the `format!` family of macros, there are
    /// two forms of `panic!`: if there are no parameters given, the first argument
    /// is not a format string and used literally. So while `format!("{}")` will
    /// fail to compile, `panic!("{}")` will not.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```no_run
    /// panic!("This `panic!` is probably missing a parameter there: {}");
    /// ```
    pub PANIC_PARAMS,
    style,
    "missing parameters in `panic!` calls"
}

declare_clippy_lint! {
    /// **What it does:** Checks for usage of `unimplemented!`.
    ///
    /// **Why is this bad?** This macro should not be present in production code
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```no_run
    /// unimplemented!();
    /// ```
    pub UNIMPLEMENTED,
    restriction,
    "`unimplemented!` should not be present in production code"
}

declare_lint_pass!(PanicUnimplemented => [PANIC_PARAMS, UNIMPLEMENTED]);

impl EarlyLintPass for PanicUnimplemented {
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &ast::Mac) {
        if mac.node.path == sym!(unimplemented) {
            span_lint(
                cx,
                UNIMPLEMENTED,
                mac.span,
                "`unimplemented` should not be present in production code",
            );
        } else if mac.node.path == sym!(panic) || mac.node.path == sym!(assert) {
            let tts = mac.node.tts.clone();
            let mut parser = parser::Parser::new(&cx.sess.parse_sess, tts, None, false, false, None);

            if mac.node.path == sym!(assert) {
                if parser.parse_expr().map_err(|mut err| err.cancel()).is_err() {
                    return;
                }

                if parser.expect(&token::Comma).map_err(|mut err| err.cancel()).is_err() {
                    return;
                }
            }

            if_chain! {
                if let Ok((string, _)) = parser.parse_str();
                let span = parser.prev_span;
                let string = string.as_str().replace("{{", "").replace("}}", "");
                if let Some(par) = string.find('{');
                if string[par..].contains('}');
                if parser.expect(&token::Comma).map_err(|mut err| err.cancel()).is_err();
                then {
                    span_lint(cx, PANIC_PARAMS, span,
                      "you probably are missing some parameter in your format string");
                }
            }
        }
    }
}
