use crate::utils::span_lint_and_help;
use if_chain::if_chain;
use rustc_ast::ast::{Item, MacArgs, MacCall};
use rustc_ast::token;
use rustc_ast::tokenstream::TokenTree;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// **What it does:**
    /// Lint {debug_}assert{_eq,_ne}! without a custom panic message.
    ///
    /// **Why is this bad?**
    /// If the assertion fails, a custom message may make it easier to debug what went wrong.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// # fn some_condition(_x: u8) -> bool { true }
    /// # fn bar(x: u8) -> u8 { x }
    /// # let foo = 0u8;
    /// # let a = 0u8;
    /// # let b = 0u8;
    /// assert!(some_condition(foo));
    /// debug_assert_eq(a, bar(b));
    /// ```
    /// Use instead:
    /// ```rust
    /// # fn some_condition(_x: u8) -> bool { true }
    /// # fn bar(x: u8) -> u8 { x }
    /// # let foo = 0u8;
    /// # let a = 0u8;
    /// # let b = 0u8;
    /// assert!(some_condition(foo), "foo failed some condition: foo = {}", foo);
    /// debug_assert_eq!(a, bar(b), "failed to find inverse of bar at {}", a);
    /// ```
    pub UNINFORMATIVE_ASSERTS,
    pedantic,
    "using `assert!` without custom panic message"
}

#[derive(Default)]
pub struct UninformativeAsserts {
    test_fns_deep: u32,
}

impl_lint_pass!(UninformativeAsserts => [UNINFORMATIVE_ASSERTS]);

const ONE_ARG_ASSERT: [&str; 2] = ["assert", "debug_assert"];
const TWO_ARG_ASSERT: [&str; 4] = ["assert_eq", "assert_ne", "debug_assert_eq", "debug_assert_ne"];

impl EarlyLintPass for UninformativeAsserts {
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        if self.test_fns_deep > 0 {
            return;
        }
        if let MacArgs::Delimited(_, _, ts) = &*mac.args {
            let args_tts = ts.trees().collect::<Vec<_>>();
            if let [seg] = &*mac.path.segments {
                let mac_name = seg.ident.name.as_str();
                let msg_arg_idx = if ONE_ARG_ASSERT.contains(&&*mac_name) {
                    1
                } else if TWO_ARG_ASSERT.contains(&&*mac_name) {
                    2
                } else {
                    return;
                };
                // this is a call to an `assert!`-family macro
                // check if it has a custom panic message argument
                let opt_msg = args_tts.split(is_comma).nth(msg_arg_idx);
                if let None | Some([]) = opt_msg {
                    span_lint_and_help(
                        cx,
                        UNINFORMATIVE_ASSERTS,
                        mac.span(),
                        "`assert!` called without custom panic message",
                        None,
                        "consider adding a custom panic message",
                    );
                }
            }
        }
    }

    fn check_item(&mut self, _: &EarlyContext<'_>, item: &Item) {
        if item.attrs.iter().any(|attr| attr.has_name(sym!(test))) {
            self.test_fns_deep = self.test_fns_deep.saturating_add(1);
        }
    }

    fn check_item_post(&mut self, _: &EarlyContext<'_>, item: &Item) {
        if item.attrs.iter().any(|attr| attr.has_name(sym!(test))) {
            self.test_fns_deep = self.test_fns_deep.saturating_sub(1);
        }
    }
}

fn is_comma(tt: &TokenTree) -> bool {
    if_chain! {
        if let TokenTree::Token(token) = tt;
        if let token::TokenKind::Comma = token.kind;
        then {
            return true;
        }
    }
    false
}
