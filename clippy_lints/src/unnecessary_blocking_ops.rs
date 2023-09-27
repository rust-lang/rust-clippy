use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::visitors::for_each_expr_with_closures;
use clippy_utils::{def_path_def_ids, fn_def_id, is_lint_allowed};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::hir_id::CRATE_HIR_ID;
use rustc_hir::{Body, ExprKind, GeneratorKind, HirIdSet};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for async function or async closure with blocking operations that
    /// could be replaced with their async counterpart.
    ///
    /// ### Why is this bad?
    /// Blocking a thread prevents tasks being swapped, causing other tasks to stop running
    /// until the thread is no longer blocked, which might not be as expected in an async context.
    ///
    /// ### Known problems
    /// Not all blocking operations can be detected, as for now, this lint only detects a small
    /// set of functions in standard library by default. And some of the suggestions might need
    /// additional features to work properly.
    ///
    /// ### Example
    /// ```rust
    /// use std::time::Duration;
    /// pub async fn foo() {
    ///     std::thread::sleep(Duration::from_secs(5));
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::time::Duration;
    /// pub async fn foo() {
    ///     tokio::time::sleep(Duration::from_secs(5));
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub UNNECESSARY_BLOCKING_OPS,
    nursery,
    "blocking operations in an async context"
}

pub(crate) struct UnnecessaryBlockingOps {
    blocking_ops: Vec<String>,
    blocking_ops_with_suggs: Vec<[String; 2]>,
    /// Map of resolved funtion def_id with suggestion string after checking crate
    id_with_suggs: FxHashMap<DefId, String>,
    /// Keep track of visited block ids to skip checking the same bodies in `check_body` calls
    visited_block: HirIdSet,
}

impl UnnecessaryBlockingOps {
    pub(crate) fn new(blocking_ops: Vec<String>, blocking_ops_with_suggs: Vec<[String; 2]>) -> Self {
        Self {
            blocking_ops,
            blocking_ops_with_suggs,
            id_with_suggs: FxHashMap::default(),
            visited_block: HirIdSet::default(),
        }
    }
}

impl_lint_pass!(UnnecessaryBlockingOps => [UNNECESSARY_BLOCKING_OPS]);

// TODO: Should we throw away all suggestions and and give full control to user configurations?
// this feels like a free ad for tokio :P
static HARD_CODED_BLOCKING_OPS_WITH_SUGG: [[&str; 2]; 26] = [
    // Sleep
    ["std::thread::sleep", "tokio::time::sleep"],
    // IO functions
    ["std::io::copy", "tokio::io::copy"],
    ["std::io::empty", "tokio::io::empty"],
    ["std::io::repeat", "tokio::io::repeat"],
    ["std::io::sink", "tokio::io::sink"],
    ["std::io::stderr", "tokio::io::stderr"],
    ["std::io::stdin", "tokio::io::stdin"],
    ["std::io::stdout", "tokio::io::stdout"],
    // Filesystem functions
    ["std::fs::try_exists", "tokio::fs::try_exists"],
    ["std::fs::canonicalize", "tokio::fs::canonicalize"],
    ["std::fs::copy", "tokio::fs::copy"],
    ["std::fs::create_dir", "tokio::fs::create_dir"],
    ["std::fs::create_dir_all", "tokio::fs::create_dir_all"],
    ["std::fs::hard_link", "tokio::fs::hard_link"],
    ["std::fs::metadata", "tokio::fs::metadata"],
    ["std::fs::read", "tokio::fs::read"],
    ["std::fs::read_dir", "tokio::fs::read_dir"],
    ["std::fs::read_to_string", "tokio::fs::read_to_string"],
    ["std::fs::remove_dir", "tokio::fs::remove_dir"],
    ["std::fs::remove_dir_all", "tokio::fs::remove_dir_all"],
    ["std::fs::remove_file", "tokio::fs::remove_file"],
    ["std::fs::rename", "tokio::fs::rename"],
    ["std::fs::set_permissions", "tokio::fs::set_permissions"],
    ["std::fs::soft_link", "tokio::fs::soft_link"],
    ["std::fs::symlink_metadata", "tokio::fs::symlink_metadata"],
    ["std::fs::write", "tokio::fs::write"],
];

impl<'tcx> LateLintPass<'tcx> for UnnecessaryBlockingOps {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // Avoids processing and storing a long list of paths if this lint was allowed entirely
        if is_lint_allowed(cx, UNNECESSARY_BLOCKING_OPS, CRATE_HIR_ID) {
            return;
        }

        let full_fn_list = HARD_CODED_BLOCKING_OPS_WITH_SUGG
            .into_iter()
            // Chain configured functions without suggestions
            .chain(self.blocking_ops.iter().map(|p| [p, ""]))
            // Chain configured functions with suggestions
            .chain(
                self.blocking_ops_with_suggs
                    .iter()
                    .map(|[p, s]| [p.as_str(), s.as_str()]),
            );

        for [path_str, sugg_path_str] in full_fn_list {
            let path = path_str.split("::").collect::<Vec<_>>();
            for did in def_path_def_ids(cx, &path) {
                self.id_with_suggs.insert(did, sugg_path_str.to_string());
            }
        }
    }

    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
        if self.visited_block.contains(&body.value.hir_id) {
            return;
        }
        if let Some(GeneratorKind::Async(_)) = body.generator_kind() {
            for_each_expr_with_closures(cx, body, |ex| {
                if let ExprKind::Block(block, _) = ex.kind {
                    self.visited_block.insert(block.hir_id);
                } else if let Some(call_did) = fn_def_id(cx, ex) &&
                    let Some(replace_sugg) = self.id_with_suggs.get(&call_did)
                {
                    span_lint_and_then(
                        cx,
                        UNNECESSARY_BLOCKING_OPS,
                        ex.span,
                        "blocking function call detected in an async body",
                        |diag| {
                            if !replace_sugg.is_empty() {
                                diag.span_suggestion(
                                    ex.span,
                                    "try using an async counterpart such as",
                                    replace_sugg,
                                    Applicability::Unspecified,
                                );
                            }
                        }
                    );
                }
                ControlFlow::<()>::Continue(())
            });
        }
    }
}
