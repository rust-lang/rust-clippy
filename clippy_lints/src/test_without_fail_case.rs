use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::macros::root_macro_call_first_node;
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
use rustc_session::impl_lint_pass;
use rustc_span::sym;

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
    "test function cannot fail because it does not call anywhere to panic or assert"
}

pub struct TestWithoutFailCase {
    config: SearchConfig,
}

impl TestWithoutFailCase {
    pub fn new(conf: &Conf) -> Self {
        Self {
            config: SearchConfig {
                indexing_fallible: conf.test_without_fail_case_include_indexing_as_fallible,
                fallible_paths: conf.test_without_fail_case_fallible_paths.iter().cloned().collect(),
                non_fallible_paths: conf.test_without_fail_case_non_fallible_paths.iter().cloned().collect(),
            },
        }
    }
}

impl_lint_pass!(TestWithoutFailCase => [TEST_WITHOUT_FAIL_CASE]);

impl<'tcx> LateLintPass<'tcx> for TestWithoutFailCase {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only interested in functions that are annotated with `#[test]`.
        if let ItemKind::Fn(_, _, body_id) = item.kind
            && is_in_test_function(cx.tcx, item.hir_id())
        {
            let body = cx.tcx.hir().body(body_id);
            let typeck_results = cx.tcx.typeck(item.owner_id);
            let fail_found = SearchFailIntraFunction::find_fail(cx, typeck_results, &self.config, body);
            if !fail_found {
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

/// Set of options that user provivdes through configs, to modify the lint behaviour
/// according to their repo.
struct SearchConfig {
    /// If search should consider indexing as fallible.
    indexing_fallible: bool,
    /// Set of paths that are marked as fallible.
    fallible_paths: FxHashSet<String>,
    /// Set of paths that are marked as non fallible.
    non_fallible_paths: FxHashSet<String>,
}

/// Visitor that searches for expressions that could cause a panic, such as `panic!`,
/// `assert!`, `unwrap()`, or calls to functions that can panic.
struct SearchFailIntraFunction<'a, 'tcx> {
    /// The lint context
    cx: &'a LateContext<'tcx>,
    /// Whether a way to fail is found or not.
    fail_found: bool,
    /// Type checking results for the current body
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
    /// Set of function `DefId`s that have been visited to avoid infinite recursion
    visited_functions: FxHashSet<DefId>,
    /// Search configs containing the set of user provided configurations.
    search_config: &'a SearchConfig,
}

impl<'a, 'tcx> SearchFailIntraFunction<'a, 'tcx> {
    pub fn new(
        cx: &'a LateContext<'tcx>,
        typeck_results: &'tcx ty::TypeckResults<'tcx>,
        search_config: &'a SearchConfig,
    ) -> Self {
        Self {
            cx,
            fail_found: false,
            typeck_results,
            visited_functions: FxHashSet::default(),
            search_config,
        }
    }

    /// Searches for a way to panic in the given body and returns the span if found
    pub fn find_fail(
        cx: &'a LateContext<'tcx>,
        typeck_results: &'tcx ty::TypeckResults<'tcx>,
        search_config: &'a SearchConfig,
        body: impl Visitable<'tcx>,
    ) -> bool {
        let mut visitor = Self::new(cx, typeck_results, search_config);
        body.visit(&mut visitor);
        visitor.fail_found
    }

    /// Checks the called function to see if it contains a panic
    fn check_called_function(&mut self, def_id: DefId) {
        // Avoid infinite recursion by checking if we've already visited this function
        if !self.visited_functions.insert(def_id) {
            return;
        }

        if def_id.is_local() {
            let hir = self.cx.tcx.hir();
            if let Some(local_def_id) = def_id.as_local() {
                if let Some(body) = hir.maybe_body_owned_by(local_def_id) {
                    let typeck_results = self.cx.tcx.typeck(local_def_id);
                    let mut new_visitor = SearchFailIntraFunction {
                        cx: self.cx,
                        fail_found: false,
                        typeck_results,
                        visited_functions: self.visited_functions.clone(),
                        search_config: &self.search_config,
                    };
                    body.visit(&mut new_visitor);
                    if new_visitor.fail_found {
                        self.fail_found = true;
                    }
                }
            }
        } else {
            // For external functions, assume they can panic
            self.fail_found = true;
        }
    }
}

impl<'tcx> Visitor<'tcx> for SearchFailIntraFunction<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        if self.fail_found {
            // If we've already found a panic, no need to continue
            return;
        }

        match expr.kind {
            ExprKind::Index(slice_expr, index_expr, _) => {
                // If indexing into slices is considered fallible, we treat it as a potential failure point
                if self.search_config.indexing_fallible {
                    self.fail_found = true;
                    return;
                }
                self.visit_expr(slice_expr);
                self.visit_expr(index_expr);
            },
            ExprKind::Call(callee, args) => {
                if let ExprKind::Path(ref qpath) = callee.kind {
                    if let Res::Def(_, def_id) = self.cx.qpath_res(qpath, callee.hir_id) {
                        self.check_called_function(def_id);
                        if self.fail_found {
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
                    self.check_called_function(def_id);
                    if self.fail_found {
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
                    let macro_with_path = self.cx.tcx.def_path_str(macro_call.def_id);
                    // Skip macros that are defined as `non_fallible` in the clippy.toml file.
                    // Some examples that would fit here can be `println!`, `print!`, `eprintln!`,
                    // `eprint!`. This is a special case, these macros can panic, but it is very
                    // unlikely that this is intended as the tests assertion. In the name of
                    // reducing false negatives we are giving out soundness.
                    //
                    // This reduces the number of false positives
                    if self.search_config.non_fallible_paths.contains(&macro_with_path) {
                        return;
                    }

                    if self.search_config.fallible_paths.contains(&macro_with_path) {
                        self.fail_found = true;
                        return;
                    }
                }

                // TODO: also make these two configurable.
                // Check for `unwrap` and `expect` method calls
                if let Some(arglists) = method_chain_args(expr, &["unwrap"]).or(method_chain_args(expr, &["expect"])) {
                    let receiver_ty = self.typeck_results.expr_ty(arglists[0].0).peel_refs();
                    if is_type_diagnostic_item(self.cx, receiver_ty, sym::Option)
                        || is_type_diagnostic_item(self.cx, receiver_ty, sym::Result)
                    {
                        self.fail_found = true;
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
