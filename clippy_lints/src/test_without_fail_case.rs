use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::macros::{is_panic, root_macro_call_first_node};
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::Visitable;
use clippy_utils::{is_in_test_function, method_chain_args};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{AnonConst, Expr, ExprKind, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for test functions that cannot fail.
    ///
    /// ### Why is this bad?
    /// A test function that cannot fail might indicate that it does not actually test anything.
    /// It could lead to false positives in test suites, giving a false sense of security.
    ///
    /// ### Example
    /// ```rust
    /// #[test]
    /// fn my_test() {
    ///     let x = 2;
    ///     let y = 2;
    ///     if x + y != 4 {
    ///         eprintln!("this is not a correct test")
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// #[test]
    /// fn my_test() {
    ///     let x = 2;
    ///     let y = 2;
    ///     assert_eq!(x + y, 4);
    /// }
    /// ```
    #[clippy::version = "1.82.0"]
    pub TEST_WITHOUT_FAIL_CASE,
    restriction,
    "test function cannot fail because it does not panic or assert"
}

declare_lint_pass!(TestWithoutFailCase => [TEST_WITHOUT_FAIL_CASE]);

impl<'tcx> LateLintPass<'tcx> for TestWithoutFailCase {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only interested in functions that are annotated with `#[test]`.
        if let ItemKind::Fn(_, _, body_id) = item.kind
            && is_in_test_function(cx.tcx, item.hir_id())
        {
            let body = cx.tcx.hir().body(body_id);
            let typeck_results = cx.tcx.typeck(item.owner_id);
            let panic_span = SearchPanicIntraFunction::find_span(cx, typeck_results, body);
            if panic_span.is_none() {
                // No way to panic for this test function
                span_lint_and_note(
                    cx,
                    TEST_WITHOUT_FAIL_CASE,
                    item.span,
                    "this function marked with `#[test]` cannot fail and will always succeed",
                    None,
                    "consider adding assertions or panics to test failure cases",
                );
            }
        }
    }
}

/// Visitor that searches for expressions that could cause a panic, such as `panic!`,
/// `assert!`, `unwrap()`, or calls to functions that can panic.
struct SearchPanicIntraFunction<'a, 'tcx> {
    /// The lint context
    cx: &'a LateContext<'tcx>,
    /// The span where a panic was found
    panic_span: Option<Span>,
    /// Type checking results for the current body
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
    /// Set of function `DefId`s that have been visited to avoid infinite recursion
    visited_functions: FxHashSet<DefId>,
}

impl<'a, 'tcx> SearchPanicIntraFunction<'a, 'tcx> {
    /// Creates a new `FindPanicUnwrap` visitor
    pub fn new(cx: &'a LateContext<'tcx>, typeck_results: &'tcx ty::TypeckResults<'tcx>) -> Self {
        Self {
            cx,
            panic_span: None,
            typeck_results,
            visited_functions: FxHashSet::default(),
        }
    }

    /// Searches for a way to panic in the given body and returns the span if found
    pub fn find_span(
        cx: &'a LateContext<'tcx>,
        typeck_results: &'tcx ty::TypeckResults<'tcx>,
        body: impl Visitable<'tcx>,
    ) -> Option<Span> {
        let mut visitor = Self::new(cx, typeck_results);
        body.visit(&mut visitor);
        visitor.panic_span
    }

    /// Checks the called function to see if it contains a panic
    fn check_called_function(&mut self, def_id: DefId, span: Span) {
        // Avoid infinite recursion by checking if we've already visited this function
        if !self.visited_functions.insert(def_id) {
            return;
        }

        if def_id.is_local() {
            let hir = self.cx.tcx.hir();
            if let Some(local_def_id) = def_id.as_local() {
                if let Some(body) = hir.maybe_body_owned_by(local_def_id) {
                    let typeck_results = self.cx.tcx.typeck(local_def_id);
                    let mut new_visitor = SearchPanicIntraFunction {
                        cx: self.cx,
                        panic_span: None,
                        typeck_results,
                        visited_functions: self.visited_functions.clone(),
                    };
                    body.visit(&mut new_visitor);
                    if let Some(panic_span) = new_visitor.panic_span {
                        self.panic_span = Some(panic_span);
                    }
                }
            }
        } else {
            // For external functions, assume they can panic
            self.panic_span = Some(span);
        }
    }
}

impl<'tcx> Visitor<'tcx> for SearchPanicIntraFunction<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        if self.panic_span.is_some() {
            // If we've already found a panic, no need to continue
            return;
        }

        match expr.kind {
            ExprKind::Call(callee, args) => {
                if let ExprKind::Path(ref qpath) = callee.kind {
                    if let Res::Def(_, def_id) = self.cx.qpath_res(qpath, callee.hir_id) {
                        self.check_called_function(def_id, expr.span);
                        if self.panic_span.is_some() {
                            return;
                        }
                    }
                }
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            },
            ExprKind::MethodCall(_, receiver, args, _) => {
                if let Some(def_id) = self.typeck_results.type_dependent_def_id(expr.hir_id) {
                    self.check_called_function(def_id, expr.span);
                    if self.panic_span.is_some() {
                        return;
                    }
                }
                self.visit_expr(receiver);
                for arg in args {
                    self.visit_expr(arg);
                }
            },
            _ => {
                if let Some(macro_call) = root_macro_call_first_node(self.cx, expr) {
                    let macro_name = self.cx.tcx.item_name(macro_call.def_id);
                    // Skip macros like `println!`, `print!`, `eprintln!`, `eprint!`.
                    // This is a special case, these macros can panic, but it is very unlikely
                    // that this is intended. In the name of reducing false positiveness we are
                    // giving out soundness.
                    //
                    // This decision can be justified as it is highly unlikely that the tool is sound
                    // without this additional check, and with this we are reducing the number of false
                    // positives.
                    if matches!(macro_name.as_str(), "println" | "print" | "eprintln" | "eprint" | "dbg") {
                        return;
                    }
                    if is_panic(self.cx, macro_call.def_id)
                        || matches!(macro_name.as_str(), "assert" | "assert_eq" | "assert_ne")
                    {
                        self.panic_span = Some(macro_call.span);
                        return;
                    }
                }

                // Check for `unwrap` and `expect` method calls
                if let Some(arglists) = method_chain_args(expr, &["unwrap"]).or(method_chain_args(expr, &["expect"])) {
                    let receiver_ty = self.typeck_results.expr_ty(arglists[0].0).peel_refs();
                    if is_type_diagnostic_item(self.cx, receiver_ty, sym::Option)
                        || is_type_diagnostic_item(self.cx, receiver_ty, sym::Result)
                    {
                        self.panic_span = Some(expr.span);
                        return;
                    }
                }

                intravisit::walk_expr(self, expr);
            },
        }
    }

    // Do not visit anonymous constants, as panics in const contexts are compile-time errors
    fn visit_anon_const(&mut self, _: &'tcx AnonConst) {}

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }
}
