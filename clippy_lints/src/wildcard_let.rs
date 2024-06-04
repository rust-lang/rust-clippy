use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::{trim_span, walk_span_to_context};
use rustc_ast::ast::{Expr, ExprKind, LitKind, Pat, PatKind, RangeEnd, RangeLimits};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_ast::ast::Local;

declare_clippy_lint! {
	/// ### What it does
	/// check for `let _ = ...`.
	///
	/// this may be used by crates that with to force `#[must_use]`
	/// values to actually used, along with `#[forbid(unused_must_use)]`.
	pub WILDCARD_LET,
	restriction,
	"wildcard let"
}
impl_lint_pass!(WildcardLet => [WILDCARD_LET]);

pub struct WildcardLet {}

impl EarlyLintPass for WildcardLet {
	fn check_local(&mut self, cx: &EarlyContext<'_>, local: &Local) {
		let span = local.pat.span;
		if in_external_macro(cx.sess(), span) {
			return
		}
		if let PatKind::Wild =  local.pat.kind {
			span_lint_and_help(
				cx, WILDCARD_LET, span, "wildcard let", None,
				"remove this binding or handle the value");
		}
	}
}
