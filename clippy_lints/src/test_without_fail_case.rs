use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{is_panic, root_macro_call_first_node};
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::Visitable;
use clippy_utils::{is_in_test_function, method_chain_args};
use rustc_hir::intravisit::{self, FnKind, Visitor};
use rustc_hir::{AnonConst, Body, Expr, FnDecl, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Triggers when a testing function (marked with the `#[test]` attribute) does not have a way to fail.
    ///
    /// ### Why restrict this?
    /// If a test does not have a way to fail, the developer might be getting false positives from their test suites.
    /// The idiomatic way of using test functions should be such that they actually can fail in an erroneous state.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn my_cool_test() {
    ///     // [...]
    /// }
    ///
    /// #[cfg(test)]
    /// mod tests {
    ///     // [...]
    /// }
    ///
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[cfg(test)]
    /// mod tests {
    ///     #[test]
    ///     fn my_cool_test() {
    ///         // [...]
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub TEST_WITHOUT_FAIL_CASE,
    restriction,
    "A test function is outside the testing module."
}

declare_lint_pass!(TestWithoutFailCase => [TEST_WITHOUT_FAIL_CASE]);

/*
impl<'tcx> LateLintPass<'tcx> for TestWithoutFailCase {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'_>) {
        if let rustc_hir::ItemKind::Fn(sig, _, body_id) = item.kind {

        }
        if is_in_test_function(cx.tcx, item.hir_id()) {
            let typck = cx.tcx.typeck(item.id.owner_id.def_id);
            let find_panic_visitor = FindPanicUnwrap::find_span(cx, typck, item.body)
            span_lint(cx, TEST_WITHOUT_FAIL_CASE, item.span, "test function cannot panic");
        }
    }
}
    */

impl<'tcx> LateLintPass<'tcx> for TestWithoutFailCase {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Fn(_, _, body_id) = item.kind
            && is_in_test_function(cx.tcx, item.hir_id())
        {
            let body = cx.tcx.hir().body(body_id);
            let typ = cx.tcx.typeck(item.owner_id);
            let panic_span = FindPanicUnwrap::find_span(cx, typ, body);
            if panic_span.is_none() {
                // No way to panic for this test.
                #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
                span_lint_and_then(
                    cx,
                    TEST_WITHOUT_FAIL_CASE,
                    item.span,
                    "this function marked with #[test] has no way to fail",
                    |diag| {
                        diag.note("make sure that something is checked in this test");
                    },
                );
            }
        }
    }
}

struct FindPanicUnwrap<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    is_const: bool,
    panic_span: Option<Span>,
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
}

impl<'a, 'tcx> FindPanicUnwrap<'a, 'tcx> {
    pub fn find_span(
        cx: &'a LateContext<'tcx>,
        typeck_results: &'tcx ty::TypeckResults<'tcx>,
        body: impl Visitable<'tcx>,
    ) -> Option<(Span, bool)> {
        let mut vis = Self {
            cx,
            is_const: false,
            panic_span: None,
            typeck_results,
        };
        body.visit(&mut vis);
        vis.panic_span.map(|el| (el, vis.is_const))
    }
}

impl<'a, 'tcx> Visitor<'tcx> for FindPanicUnwrap<'a, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        if self.panic_span.is_some() {
            return;
        }

        if let Some(macro_call) = root_macro_call_first_node(self.cx, expr) {
            if is_panic(self.cx, macro_call.def_id)
                || matches!(
                    self.cx.tcx.item_name(macro_call.def_id).as_str(),
                    "assert" | "assert_eq" | "assert_ne"
                )
            {
                self.is_const = self.cx.tcx.hir().is_inside_const_context(expr.hir_id);
                self.panic_span = Some(macro_call.span);
            }
        }

        // check for `unwrap` and `expect` for both `Option` and `Result`
        if let Some(arglists) = method_chain_args(expr, &["unwrap"]).or(method_chain_args(expr, &["expect"])) {
            let receiver_ty = self.typeck_results.expr_ty(arglists[0].0).peel_refs();
            if is_type_diagnostic_item(self.cx, receiver_ty, sym::Option)
                || is_type_diagnostic_item(self.cx, receiver_ty, sym::Result)
            {
                self.panic_span = Some(expr.span);
            }
        }

        // and check sub-expressions
        intravisit::walk_expr(self, expr);
    }

    // Panics in const blocks will cause compilation to fail.
    fn visit_anon_const(&mut self, _: &'tcx AnonConst) {}

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }
}
