use clippy_config::Conf;
use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{is_in_cfg_test, is_test_function};
use rustc_errors::Applicability;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{self as hir, Body, ExprKind, FnDecl};
use rustc_lexer::is_ident;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::{Span, Symbol, edition};
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
    /// #[cfg(test)]
    /// mod tests {
    ///   use super::*;
    ///
    ///   #[test]
    ///   fn test_use_case() {
    ///       // test code
    ///   }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[cfg(test)]
    /// mod tests {
    ///   use super::*;
    ///
    ///   #[test]
    ///   fn use_case() {
    ///       // test code
    ///   }
    /// }
    /// ```
    #[clippy::version = "1.88.0"]
    pub REDUNDANT_TEST_PREFIX,
    restriction,
    "redundant `test_` prefix in test function name"
}

pub struct RedundantTestPrefix {
    check_outside_test_cfg: bool,
}

impl RedundantTestPrefix {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            check_outside_test_cfg: conf.redundant_test_prefix_check_outside_cfg_test,
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
        fn_def_id: LocalDefId,
    ) {
        // Ignore methods and closures.
        let FnKind::ItemFn(ref ident, ..) = kind else {
            return;
        };

        // Skip the lint if the function is within a macro expansion.
        if ident.span.from_expansion() {
            return;
        }

        // Skip if the function name does not start with `test_`.
        if !ident.as_str().starts_with("test_") {
            return;
        }

        // Skip the lint if the function is not within a node marked with `#[cfg(test)]` attribute.
        // Continue if the function is inside the node marked with `#[cfg(test)]` or lint is enforced
        // via configuration (most likely to include integration tests in lint's scope).
        if !(self.check_outside_test_cfg || is_in_cfg_test(cx.tcx, body.id().hir_id)) {
            return;
        }

        if is_test_function(cx.tcx, cx.tcx.local_def_id_to_hir_id(fn_def_id)) {
            let msg = "redundant `test_` prefix in test function name";
            let mut non_prefixed = ident.as_str().trim_start_matches("test_").to_string();

            if is_invalid_ident(&non_prefixed) {
                // If the prefix-trimmed name is not a valid function name, do not provide an
                // automatic fix, just suggest renaming the function.
                span_lint_and_help(
                    cx,
                    REDUNDANT_TEST_PREFIX,
                    ident.span,
                    msg,
                    None,
                    "consider function renaming (just removing `test_` prefix will produce invalid function name)",
                );
            } else if name_conflicts(cx, body, &non_prefixed) {
                // If `non_prefixed` conflicts with another function in the same module/scope,
                // do not provide an automatic fix, but still emit a fix suggestion.
                non_prefixed.push_str("_works");
                span_lint_and_sugg(
                    cx,
                    REDUNDANT_TEST_PREFIX,
                    ident.span,
                    msg,
                    "consider function renaming (just removing `test_` prefix will cause a name conflict)",
                    non_prefixed,
                    Applicability::MaybeIncorrect,
                );
            } else {
                // If `non_prefixed` is a valid identifier and does not conflict with another function,
                // so we can suggest an auto-fix.
                span_lint_and_sugg(
                    cx,
                    REDUNDANT_TEST_PREFIX,
                    ident.span,
                    msg,
                    "consider removing the `test_` prefix",
                    non_prefixed,
                    Applicability::MaybeIncorrect,
                );
            }
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
    let (module, _module_span, _module_hir) = tcx.hir_get_module(tcx.parent_module(id));
    for item in module.item_ids {
        let item = tcx.hir_item(*item);
        if let hir::ItemKind::Fn { ident, .. } = item.kind
            && ident.name.as_str() == fn_name
        {
            // Name conflict found
            return true;
        }
    }

    // Also check that within the body of the function there is also no function call
    // with the same name (since it will result in recursion)
    for_each_expr(cx, body, |expr| {
        if let ExprKind::Path(qpath) = &expr.kind
            && let Some(def_id) = cx.qpath_res(qpath, expr.hir_id).opt_def_id()
            && let Some(name) = tcx.opt_item_name(def_id)
            && name.as_str() == fn_name
        {
            // Function call with the same name found
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    })
    .is_some()
}

fn is_invalid_ident(ident: &str) -> bool {
    // The identifier is either a reserved keyword, or starts with an invalid sequence.
    Symbol::intern(ident).is_reserved(|| edition::LATEST_STABLE_EDITION) || !is_ident(ident)
}
