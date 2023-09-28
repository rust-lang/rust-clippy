use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::visitors::for_each_expr_with_closures;
use clippy_utils::{def_path_def_ids, fn_def_id, is_lint_allowed};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::{Applicability, Diagnostic};
use rustc_hir::def_id::DefId;
use rustc_hir::hir_id::CRATE_HIR_ID;
use rustc_hir::{Body, Expr, ExprKind, GeneratorKind, HirIdSet};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;

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
    /// ```ignore
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
    id_with_suggs: FxHashMap<DefId, Option<String>>,
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

static HARD_CODED_BLOCKING_OPS: [&[&str]; 21] = [
    &["std", "thread", "sleep"],
    // Filesystem functions
    &["std", "fs", "try_exists"],
    &["std", "fs", "canonicalize"],
    &["std", "fs", "copy"],
    &["std", "fs", "create_dir"],
    &["std", "fs", "create_dir_all"],
    &["std", "fs", "hard_link"],
    &["std", "fs", "metadata"],
    &["std", "fs", "read"],
    &["std", "fs", "read_dir"],
    &["std", "fs", "read_link"],
    &["std", "fs", "read_to_string"],
    &["std", "fs", "remove_dir"],
    &["std", "fs", "remove_dir_all"],
    &["std", "fs", "remove_file"],
    &["std", "fs", "rename"],
    &["std", "fs", "set_permissions"],
    &["std", "fs", "symlink_metadata"],
    &["std", "fs", "write"],
    // IO functions
    &["std", "io", "copy"],
    &["std", "io", "read_to_string"],
];

impl<'tcx> LateLintPass<'tcx> for UnnecessaryBlockingOps {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // Avoids processing and storing a long list of paths if this lint was allowed entirely
        if is_lint_allowed(cx, UNNECESSARY_BLOCKING_OPS, CRATE_HIR_ID) {
            return;
        }

        let full_fn_list = HARD_CODED_BLOCKING_OPS
            .into_iter()
            .map(|p| (p.to_vec(), None))
            // Chain configured functions without suggestions
            .chain(
                self.blocking_ops
                    .iter()
                    .map(|p| (p.split("::").collect::<Vec<_>>(), None)),
            )
            // Chain configured functions with suggestions
            .chain(
                self.blocking_ops_with_suggs
                    .iter()
                    .map(|[p, s]| (p.split("::").collect::<Vec<_>>(), Some(s.as_str()))),
            );
        for (path, maybe_sugg_str) in full_fn_list {
            for did in def_path_def_ids(cx, &path) {
                self.id_with_suggs.insert(did, maybe_sugg_str.map(ToOwned::to_owned));
            }
        }
    }

    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
        if is_lint_allowed(cx, UNNECESSARY_BLOCKING_OPS, body.value.hir_id)
            || self.visited_block.contains(&body.value.hir_id)
        {
            return;
        }
        if let Some(GeneratorKind::Async(_)) = body.generator_kind() {
            for_each_expr_with_closures(cx, body, |ex| {
                match ex.kind {
                    ExprKind::Block(block, _) => {
                        self.visited_block.insert(block.hir_id);
                    }
                    ExprKind::Call(call, _)
                        if let Some(call_did) = fn_def_id(cx, ex) &&
                            let Some(maybe_sugg) = self.id_with_suggs.get(&call_did) => {
                        span_lint_and_then(
                            cx,
                            UNNECESSARY_BLOCKING_OPS,
                            call.span,
                            "blocking function call detected in an async body",
                            |diag| {
                                if let Some(sugg_fn_path) = maybe_sugg {
                                    make_suggestion(diag, cx, ex, call.span, sugg_fn_path);
                                }
                            }
                        );
                    }
                    _ => {}
                }
                ControlFlow::<()>::Continue(())
            });
        }
    }
}

fn make_suggestion(diag: &mut Diagnostic, cx: &LateContext<'_>, expr: &Expr<'_>, fn_span: Span, sugg_fn_path: &str) {
    let mut applicability = Applicability::Unspecified;
    let args_span = expr.span.with_lo(fn_span.hi());
    let args_snippet = snippet_with_applicability(cx, args_span, "..", &mut applicability);
    let suggestion = format!("{sugg_fn_path}{args_snippet}.await");
    diag.span_suggestion(
        expr.span,
        "try using its async counterpart",
        suggestion,
        Applicability::Unspecified,
    );
}
