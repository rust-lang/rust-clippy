use self::persistence::CrateInfo;
use crate::utils::conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{fn_def_id, immediate_closure_def_id, path_def_id_including_closures};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{
    def::Res,
    def_id::{DefId, DefIdMap},
    Body, Expr, ExprKind, HirId, ImplItemKind, IsAsync, ItemKind, Node, TraitItemKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;
use std::collections::VecDeque;
use std::path::PathBuf;

mod persistence;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Searches for calls to disallowed functions from an async context.
    ///
    /// ### Why is this bad?
    ///
    /// Some functions will panic when called from an async context (e.g. `block_on`), or might
    /// be too heavy and risk blocking the executor for an excessive duration.
    ///
    /// ### Example
    ///
    /// ```rust
    /// async fn foo_task() {
    ///     let handle = Handle::current();
    ///     handle.block_on(async move { println!("hello from nested async!" }));
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// async fn foo_task() {
    ///     let handle = Handle::current();
    ///     handle.spawn_blocking(move || {
    ///         handle.block_on(async move {
    ///             println!("hello from nested async!");
    ///         })
    ///     })
    /// }
    /// ```
    #[clippy::version = "1.65.0"]
    pub DISALLOWED_FROM_ASYNC,
    restriction,
    "prevent certain functions from being called from an async context"
}

const DEBUG: bool = false;

macro_rules! mprintln {
    ($($arg:tt)*) => {
        if DEBUG {
            println!($($arg)*);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DisallowedFromAsync {
    conf_disallowed: Vec<conf::DisallowedFromAsyncMethod>,
    conf_wrappers: Vec<conf::AsyncWrapperMethod>,
    disallowed_fns: Vec<ResolvedDisallowedFunction>,
    wrapper_def_ids: Vec<DefId>,
    callgraph: DefIdMap<FnInfo>,
    /// Stack of expression IDs which correspond to insulating wrapper calls.
    insulating_exprs: Vec<InsulatingExpr>,
}

impl_lint_pass!(DisallowedFromAsync => [DISALLOWED_FROM_ASYNC]);

#[derive(Debug, Clone, Copy)]
pub struct InsulatingExpr {
    /// ID of the insulating expression, used to pop from the stack in `check_expr_post`.
    expr_hir_id: HirId,
    /// `DefId` of the enclosing function for this insulator.
    ///
    /// If another function definition is nested inside the insulator expression then its
    /// def ID is checked against this value to invalidate the insulation.
    caller_def_id: DefId,
    /// Def ID of the insulator function.
    insulator_def_id: DefId,
}

#[derive(Debug, Clone)]
pub struct ResolvedDisallowedFunction {
    fn_def_id: DefId,
    /// Bottom-up call stack leading to another disallowed call (if any).
    callstack: Vec<DefId>,
}

/// Information about a function call relevant to this lint.
#[derive(Debug, Clone)]
pub struct FnCall {
    /// Call stack in bottom-up order, so that later entries represent calls to earlier entries.
    callstack: Vec<DefId>,
}

/// Information about a function relevant to this lint.
///
/// This struct is filled in gradually as the lint traverses each crate.
#[derive(Debug, Clone, Default)]
pub struct FnInfo {
    /// Whether or not this function is async.
    ///
    /// This is filled in by `check_fn` when visiting the function.
    asyncness: Option<IsAsync>,
    /// List of functions that call this function.
    callers: Vec<DefId>,
    /// Span for this function's declaration.
    decl_span: Option<Span>,
}

impl DisallowedFromAsync {
    pub fn new(
        conf_disallowed: Vec<conf::DisallowedFromAsyncMethod>,
        conf_wrappers: Vec<conf::AsyncWrapperMethod>,
    ) -> Self {
        Self {
            conf_disallowed,
            conf_wrappers,
            ..Self::default()
        }
    }

    fn record_call(&mut self, caller: DefId, callee: DefId) {
        let fn_info = self.callgraph.entry(callee).or_insert_with(FnInfo::default);
        fn_info.callers.push(caller);
    }

    fn record_fn_info(&mut self, fn_def_id: DefId, asyncness: IsAsync, span: Span) {
        let fn_info = self.callgraph.entry(fn_def_id).or_insert_with(FnInfo::default);
        fn_info.asyncness = Some(asyncness);
        fn_info.decl_span = Some(span);
    }

    fn record_disallowed(&mut self, fn_def_id: DefId) {
        // Assume that disallowed functions are blocking.
        let fn_info = self.callgraph.entry(fn_def_id).or_insert_with(FnInfo::default);
        fn_info.asyncness = Some(IsAsync::NotAsync);
    }
}

fn persistence_base_dir(cx: &LateContext<'_>) -> PathBuf {
    cx.tcx.output_filenames(()).out_directory.clone()
}

impl<'tcx> LateLintPass<'tcx> for DisallowedFromAsync {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let base_dir = persistence_base_dir(cx);

        let path_to_def_id = |segments: &[&str]| {
            if let Res::Def(_, id) = clippy_utils::def_path_res(cx, segments, None) {
                Some(id)
            } else if let Some(local_def_id) = clippy_utils::def_path_to_local_def_id(cx, segments) {
                Some(local_def_id.to_def_id())
            } else {
                None
            }
        };

        // Load saved crate info for dependencies.
        let tcx = cx.tcx;
        for crate_num in cx.tcx.crates(()) {
            let stable_crate_id = tcx.stable_crate_id(*crate_num);
            let crate_info = match CrateInfo::load(&base_dir, stable_crate_id) {
                Ok(info) => info,
                Err(e) => {
                    mprintln!("No crate info for crate {:?}: {}", crate_num, e);
                    continue;
                },
            };

            self.disallowed_fns.extend(crate_info.get_tainted_function_def_ids(tcx));
        }

        for conf in self.conf_disallowed.clone() {
            let segs: Vec<_> = conf.path().split("::").collect();
            if let Some(id) = path_to_def_id(&segs) {
                self.disallowed_fns.push(ResolvedDisallowedFunction {
                    fn_def_id: id,
                    callstack: vec![],
                });
                self.record_disallowed(id);
            } else {
                mprintln!("WARNING: unable to resolve disallowed {}", conf.path());
            }
        }

        for conf in self.conf_wrappers.clone() {
            let segs: Vec<_> = conf.path().split("::").collect();
            if let Some(id) = path_to_def_id(&segs) {
                self.wrapper_def_ids.push(id);
                self.record_disallowed(id);
            } else {
                mprintln!("WARNING: unable to resolve wrapper {}", conf.path());
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'_>) {
        /*
        mprintln!("here's the call graph:");
        for (def_id, fn_info) in &self.callgraph {
            mprintln!("[#{:?}] {:?} => {:#?}", def_id.index, def_id, fn_info);
        }
        */

        // Set of visited definition IDs, to avoid revisiting the same nodes, including for
        // recursive calls (FIXME(sproul): check this actually works for recursive calls).
        let mut visited = FxHashSet::default();

        // Persist disallowed functions to disk for use by downstream crates.
        let stable_crate_id = cx.sess().local_stable_crate_id();
        let mut persisted_crate_info = CrateInfo::new(stable_crate_id);

        for disallowed_fn in &self.disallowed_fns {
            let disallowed_def_id = disallowed_fn.fn_def_id;

            // Stack of nodes to explore.
            let mut stack = VecDeque::new();
            stack.push_back(FnCall {
                callstack: vec![disallowed_def_id],
            });

            while let Some(current_call) = stack.pop_front() {
                let current_node_id = current_call.callstack.last().copied().expect("has last caller");

                // Add to visited set, and skip this node if it was already present.
                // It's safe to avoid re-visiting nodes because any node that is in the visited
                // set is either:
                //
                // 1. Itself an insulating wrapper, implying that all calls below it are safe.
                // 2. An async function reachable from a disallowed function, in which case
                //    we have already reported an error when visiting it the first time.
                // 3. A blocking function reachable from a disallowed function, in which case we
                //    have already recorded it as tainted and added all of its callers to the
                //    search graph.
                if !visited.insert(current_node_id) {
                    continue;
                }

                // Check if current node is a known insulator, which terminates the traversal here.
                if self.wrapper_def_ids.contains(&current_node_id) {
                    mprintln!("terminating graph traversal at insulator");
                    continue;
                }

                let current_node = match self.callgraph.get(&current_node_id) {
                    Some(fn_info) => fn_info,
                    None => {
                        // Definition is missing from callgraph, this is possibly bad.
                        continue;
                    },
                };
                mprintln!("visiting node {:#?}", current_node);

                // If current node is async then there exists a path from the forbidden function
                // to this function with no insulating wrapper, so we've found a violation.
                if current_node.asyncness == Some(IsAsync::Async) {
                    let msg = "async function calls a function which is forbidden from being \
                               called in an async context";
                    let span = current_node
                        .decl_span
                        .expect("span for non-terminal function should be known");

                    span_lint_and_then(cx, DISALLOWED_FROM_ASYNC, span, &msg, |diag| {
                        let show_def_id = |def_id| {
                            let path = cx.tcx.def_path_str(def_id);
                            let span = cx.tcx.def_span(def_id);
                            format!("{} @ {:?}", path, span)
                        };

                        diag.note(&format!(
                            "calls disallowed function `{}`",
                            show_def_id(disallowed_def_id)
                        ));

                        let callstack_paths = current_call
                            .callstack
                            .iter()
                            .rev()
                            .copied()
                            .map(show_def_id)
                            .collect::<Vec<_>>();

                        diag.note(&format!("called via: {}", callstack_paths.join("\n")));

                        if !disallowed_fn.callstack.is_empty() {
                            let other_disallowed_callstack = disallowed_fn
                                .callstack
                                .iter()
                                .rev()
                                .copied()
                                .map(show_def_id)
                                .collect::<Vec<_>>();

                            diag.note(&format!(
                                "{} itself calls another disallowed function via: {}",
                                show_def_id(disallowed_def_id),
                                other_disallowed_callstack.join("\n")
                            ));
                        }
                    });
                } else {
                    // If the current function is blocking (FIXME(sproul): and public), then it needs to
                    // be recorded in the persisted crate info so that calls to it can be checked
                    // from dependent crates.
                    let def_path_hash = cx.tcx.def_path_hash(current_node_id);
                    persisted_crate_info.record_tainted_function(cx.tcx, def_path_hash, &current_call.callstack);

                    // Check all callers of this function.
                    for caller in &current_node.callers {
                        let mut fn_call = current_call.clone();
                        fn_call.callstack.push(*caller);

                        stack.push_back(fn_call);
                    }
                }
            }
        }

        // Save the persisted crate info to disk.
        if let Err(e) = persisted_crate_info.store(&persistence_base_dir(cx)) {
            panic!("error saving persisted crate info: {}", e);
        }
    }

    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
        let hir = cx.tcx.hir();
        let body_owner_def_id = hir.body_owner_def_id(body.id()).to_def_id();
        let asyncness = body.generator_kind.map_or(IsAsync::NotAsync, |_| IsAsync::Async);
        self.record_fn_info(body_owner_def_id, asyncness, body.value.span);
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // FIXME(sproul): allow recursion as per disallowed_methods
        let opt_callee = path_def_id_including_closures(cx, expr)
            .or_else(|| fn_def_id(cx, expr))
            .or_else(|| immediate_closure_def_id(cx, expr));

        let callee = match opt_callee {
            Some(def_id) => def_id,
            None => {
                return;
            },
        };

        // Calculate the enclosing caller expression (a fn decl, closure decl or generator).
        let hir = cx.tcx.hir();
        let enclosing_decl_id = hir.parent_iter(expr.hir_id).find_map(|(_, node)| {
            if let Node::Item(item) = node &&
                    let ItemKind::Fn(..) = item.kind
                {
                    Some(item.def_id.to_def_id())
                } else if let Node::TraitItem(item) = node &&
                    let TraitItemKind::Fn(..) = item.kind
                {
                    Some(item.def_id.to_def_id())
                } else if let Node::ImplItem(item) = node &&
                    let ImplItemKind::Fn(..) = item.kind
                {
                    Some(item.def_id.to_def_id())
                } else if let Node::Expr(expr) = node &&
                    let ExprKind::Closure(closure) = expr.kind
                {
                    Some(hir.body_owner_def_id(closure.body).to_def_id())
                } else {
                    None
                }
        });
        let enclosing_decl_id = match enclosing_decl_id {
            Some(decl_id) => decl_id,
            None => return,
        };

        // If the call is nested under an insulator record the caller as the insulator rather
        // than the parent function.
        let caller = if let Some(insulator) = self.insulating_exprs.last() &&
            insulator.caller_def_id == enclosing_decl_id
        {
            insulator.insulator_def_id
        } else {
            enclosing_decl_id
        };

        self.record_call(caller, callee);

        // If this expression is a call to an insulator then record it in the stack of insulating
        // expressions along with the caller ID.
        if self.wrapper_def_ids.contains(&callee) {
            self.insulating_exprs.push(InsulatingExpr {
                expr_hir_id: expr.hir_id,
                caller_def_id: caller,
                insulator_def_id: callee,
            });
        }
    }

    fn check_expr_post(&mut self, _: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if self
            .insulating_exprs
            .last()
            .map_or(false, |insulating_expr| insulating_expr.expr_hir_id == expr.hir_id)
        {
            self.insulating_exprs.pop();
        }
    }
}
