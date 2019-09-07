use rustc::lint::{LateContext, LateLintPass, LintPass};
use rustc::{declare_lint_pass, declare_tool_lint};
use rustc::hir;
use crate::utils::{span_lint};
use syntax::ast::*;
use syntax_pos::Span;
use syntax::visit::FnKind;

use super::MAP_FLATTEN_FILTERMAP;

pub(super) fn lint(cx: &LateContext<'_, '_>, expr: &hir::Expr, args: &[hir::Expr]) {
    eprintln!("LINT has been called");
    span_lint(
        cx,
        MAP_FLATTEN_FILTERMAP,
        expr.span,
        "this `.map().flatten()` can be written more simply using `.filter_map()`",
    );
}