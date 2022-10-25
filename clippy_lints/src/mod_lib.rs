use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{ptr::P, Crate, Item, ItemKind, ModKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.66.0"]
    pub MOD_LIB,
    pedantic,
    "default lint description"
}
declare_lint_pass!(ModLib => [MOD_LIB]);

impl EarlyLintPass for ModLib {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // println!("MOO Checking crate {:#?}", krate);
        check_mod(cx, &krate.items);
    }
}

fn check_mod(cx: &EarlyContext<'_>, items: &[P<Item>]) {
    for item in items {
        if let ItemKind::Mod(_, ModKind::Loaded(..)) = item.kind {
            // println!("MOO Got a Mod: {:#?}", items);
            // println!("MOO Got a Mod: {:#?}", item.ident.name);

            if item.ident.name == sym::lib {
                span_lint_and_help(
                    cx,
                    MOD_LIB,
                    item.span,
                    "uncommon use of mod::lib",
                    None,
                    "you probably meant use::package instead",
                );
            }
        }
    }
}
