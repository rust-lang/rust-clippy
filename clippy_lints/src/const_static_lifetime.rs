// use syntax::ast::*;
use rustc::lint::{LintPass, EarlyLintPass, LintArray};


declare_lint! {
    pub CONST_STATIC_LIFETIME, Warn,
    "Warn when specifying the lifetime of a constant."
}



pub struct StaticConst;


impl LintPass for StaticConst {
    fn get_lints(&self) -> LintArray {
        lint_array!(CONST_STATIC_LIFETIME)
    }
}

impl EarlyLintPass for StaticConst {}


// impl<'a,'tcx> LintPass
