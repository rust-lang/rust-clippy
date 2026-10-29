#![deny(clippy::missing_msrv_attr_impl)]
#![allow(clippy::missing_clippy_version_attribute)]
#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_hir;
#[macro_use]
extern crate rustc_lint;
extern crate rustc_middle;
use clippy_utils::msrvs::MsrvStack;
use clippy_utils::{declare_clippy_pass, extract_msrv_attr};
use rustc_hir::Expr;
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass};

declare_lint! {
    pub TEST_LINT,
    Warn,
    ""
}

struct Pass {
    msrv: MsrvStack,
}

declare_clippy_pass!(Pass => [TEST_LINT]);

impl EarlyLintPass for Pass {
    //~^ missing_msrv_attr_impl
    fn check_expr(&mut self, _: &EarlyContext<'_>, _: &rustc_ast::Expr) {}
}

fn main() {}
