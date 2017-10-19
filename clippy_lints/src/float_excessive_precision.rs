use rustc::lint::{LintArray, LintPass, EarlyLintPass, EarlyContext};
use syntax::ast::Item;

declare_lint! {
    pub FLOAT_EXCESSIVE_PRECISION,
    Warn,
    "declaring a float with too much precision"
}

pub struct FloatExcessivePrecision;

impl LintPass for FloatExcessivePrecision {
    fn get_lints(&self) -> LintArray {
        lint_array!(FLOAT_EXCESSIVE_PRECISION)
    }
}


impl EarlyLintPass for FloatExcessivePrecision {
    fn check_item(&mut self, cx: &EarlyContext, item: &Item) {}
}
