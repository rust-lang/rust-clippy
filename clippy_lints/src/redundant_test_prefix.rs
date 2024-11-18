use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{is_in_cfg_test, is_in_test_function};
use rustc_errors::Applicability;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{self as hir, Body, ExprKind, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for test functions (functions annotated with `#[test]`) that are prefixed
    /// with `test_` which is redundant.
    ///
    /// ### Why is this bad?
    /// This is redundant because test functions are already annotated with `#[test]`.
    /// Moreover, it clutters the output of `cargo test` since test functions are expanded as
    /// `module::tests::test_use_case` in the output. Without the redundant prefix, the output
    /// becomes `module::tests::use_case`, which is more readable.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn test_use_case() {
    ///     // test code
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn use_case() {
    ///    // test code
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub REDUNDANT_TEST_PREFIX,
    pedantic,
    "redundant `test_` prefix in test function name"
}

pub struct RedundantTestPrefix {
    in_integration_tests: bool,
    custom_sufix: String,
}

impl RedundantTestPrefix {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            in_integration_tests: conf.redundant_test_prefix_in_integration_tests,
            custom_sufix: conf.redundant_test_prefix_custom_suffix.clone(),
        }
    }
}

impl_lint_pass!(RedundantTestPrefix => [REDUNDANT_TEST_PREFIX]);

impl<'tcx> LateLintPass<'tcx> for RedundantTestPrefix {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'_>,
        _decl: &FnDecl<'_>,
        body: &'tcx Body<'_>,
        _span: Span,
        _fn_def_id: LocalDefId,
    ) {
        // Skip the lint if the function is not within a node with `#[cfg(test)]` attribute,
        // which is true for integration tests. If the lint is enabled for integration tests,
        // via configuration value, ignore this check.
        if !(self.in_integration_tests || is_in_cfg_test(cx.tcx, body.id().hir_id)) {
            return;
        }

        // Ignore methods and closures.
        let FnKind::ItemFn(ref ident, ..) = kind else {
            return;
        };

        if is_in_test_function(cx.tcx, body.id().hir_id) && ident.as_str().starts_with("test_") {
            let mut non_prefixed = ident.as_str().trim_start_matches("test_").to_string();
            let mut help_msg = "consider removing the `test_` prefix";
            // If `non_prefixed` conflicts with another function in the same module/scope,
            // add extra suffix to avoid name conflict.
            if name_conflicts(cx, body, &non_prefixed) {
                non_prefixed.push_str(&self.custom_sufix);
                help_msg = "consider removing the `test_` prefix (suffix avoids name conflict)";
            }
            span_lint_and_sugg(
                cx,
                REDUNDANT_TEST_PREFIX,
                ident.span,
                "redundant `test_` prefix in test function name",
                help_msg,
                non_prefixed,
                Applicability::MachineApplicable,
            );
        }
    }
}

/// Checks whether removal of the `_test` prefix from the function name will cause a name conflict.
///
/// There should be no other function with the same name in the same module/scope. Also, there
/// should not be any function call with the same name within the body of the function, to avoid
/// recursion.
fn name_conflicts<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'_>, fn_name: &str) -> bool {
    let tcx = cx.tcx;
    let id = body.id().hir_id;

    // Iterate over items in the same module/scope
    let (module, _module_span, _module_hir) = tcx.hir().get_module(tcx.parent_module(id));
    for item in module.item_ids {
        let item = tcx.hir().item(*item);
        if let hir::ItemKind::Fn(..) = item.kind {
            if item.ident.name.as_str() == fn_name {
                // Name conflict found
                return true;
            }
        }
    }

    // Also check that within the body of the function there is also no function call
    // with the same name (since it will result in recursion)
    for_each_expr(cx, body, |expr| {
        if let ExprKind::Call(path_expr, _) = expr.kind {
            if let ExprKind::Path(qpath) = &path_expr.kind {
                if let Some(def_id) = cx.qpath_res(qpath, path_expr.hir_id).opt_def_id() {
                    if let Some(name) = tcx.opt_item_name(def_id) {
                        if name.as_str() == fn_name {
                            // Function call with the same name found
                            return ControlFlow::Break(());
                        }
                    }
                }
            }
        }
        ControlFlow::Continue(())
    })
    .is_some()
}
