use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{
    FormatArgsStorage, HirNode, PanicCall, find_assert_args, format_args_inputs_span, root_macro_call,
    root_macro_call_first_node,
};
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{HasSession, SpanRangeExt};
use clippy_utils::ty::has_debug_impl;
use clippy_utils::{is_in_const_context, is_span_assert, sym};
use rustc_errors::Applicability;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `assert!(matches!(...))` / `debug_assert!(matches!(...))` which can be replaced with
    /// `core::assert_matches!(...)` / `core::debug_assert_matches!(...)`.
    ///
    /// ### Why is this bad?
    /// `assert!(matches!(...))` only communicates that the assertion failed in the panic message, while
    /// `core::assert_matches!(...)` also includes the `Debug` representation of the value being matched on.
    ///
    /// ### Example
    /// ```no_run
    /// #[derive(Debug)]
    /// enum A {
    ///     X,
    ///     Y,
    /// }
    ///
    /// fn main() {
    ///     let a = A::X;
    ///     assert!(matches!(a, A::Y));
    /// }
    /// ```
    /// produces:
    /// ```ignore
    /// assertion failed: matches!(a, A::Y)
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #[derive(Debug)]
    /// enum A {
    ///     X,
    ///     Y,
    /// }
    ///
    /// fn main() {
    ///     let a = A::X;
    ///     core::assert_matches!(a, A::Y);
    /// }
    /// ```
    /// which produces:
    /// ```ignore
    /// assertion `left matches right` failed
    ///   left: X
    ///  right: A::Y
    /// ```
    #[clippy::version = "1.98.0"]
    pub MANUAL_ASSERT_MATCHES,
    pedantic,
    "usage of `assert!(matches!(...))` or `debug_assert!(matches!(...))`"
}

impl_lint_pass!(ManualAssertMatches => [MANUAL_ASSERT_MATCHES]);

pub struct ManualAssertMatches {
    msrv: Msrv,
    format_args: FormatArgsStorage,
}

impl ManualAssertMatches {
    pub fn new(conf: &'static Conf, format_args: FormatArgsStorage) -> Self {
        Self {
            msrv: conf.msrv,
            format_args,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualAssertMatches {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &rustc_hir::Expr<'tcx>) {
        if !self.msrv.meets(cx, msrvs::ASSERT_MATCHES) {
            return;
        }

        if let Some(root_mac_call) = root_macro_call_first_node(cx, expr)
            && let Some(root_mac_name) = cx.tcx.get_diagnostic_name(root_mac_call.def_id)
            && let Some((assert_expr, panic_call)) = find_assert_args(cx, expr, root_mac_call.expn)
            && let rustc_hir::ExprKind::Match(
                match_arg,
                [
                    rustc_hir::Arm {
                        pat: match_pat,
                        guard: match_guard,
                        ..
                    },
                    _,
                ],
                _,
            ) = assert_expr.kind
            && let Some(inner_mac_call) = root_macro_call(assert_expr.span())
            && let Some(sym::matches_macro) = cx.tcx.get_diagnostic_name(inner_mac_call.def_id)
            && has_debug_impl(
                cx,
                cx.tcx.typeck(match_arg.hir_id.owner.def_id).node_type(match_arg.hir_id),
            )
            && !is_in_const_context(cx)
            && !root_mac_call.span.in_external_macro(cx.sess().source_map())
            && is_span_assert(cx, root_mac_call.span)
        {
            let assert_macro_name = match root_mac_name {
                sym::assert_macro => "assert",
                sym::debug_assert_macro => "debug_assert",
                _ => {
                    return;
                },
            };

            let Some(match_arg) = match_arg.span.get_source_text(cx) else {
                return;
            };
            let match_arg = match_arg.as_str();

            let Some(match_pat) = match_pat.span.get_source_text(cx) else {
                return;
            };
            let match_pat = match_pat.as_str();

            let guard = match match_guard {
                Some(match_guard) if let Some(source) = match_guard.span.get_source_text(cx) => {
                    let source = source.as_str();
                    format!(" if {source}")
                },
                _ => String::new(),
            };

            let format_args = match panic_call {
                PanicCall::Display(display) if let Some(source) = display.span.get_source_text(cx) => {
                    format!(", \"{{}}\", {source}")
                },
                PanicCall::Format(format_expr)
                    if let Some(args) = self.format_args.get(cx, format_expr, root_mac_call.expn)
                        && let span = format_args_inputs_span(args)
                        && let Some(source) = span.get_source_text(cx) =>
                {
                    let source = source.as_str();
                    format!(", {source}")
                },
                PanicCall::DefaultMessage => String::new(),
                _ => {
                    return;
                },
            };

            let msg = format!("manual `core::{assert_macro_name}_matches` implementation");
            let sugg = format!("core::{assert_macro_name}_matches!({match_arg}, {match_pat}{guard}{format_args})");
            span_lint_and_sugg(
                cx,
                MANUAL_ASSERT_MATCHES,
                root_mac_call.span,
                msg,
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}
