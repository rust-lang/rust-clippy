use clippy_config::types::SuggestedPath;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{def_path_def_ids, fn_def_id, is_lint_allowed};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::{Applicability, Diag};
use rustc_hir::def_id::DefId;
use rustc_hir::{
    Body, BodyId, Closure, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, ImplItem, ImplItemKind,
    Item, ItemKind, Node, TraitItem, TraitItemKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
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
    pedantic,
    "blocking operations in an async context"
}

pub(crate) struct UnnecessaryBlockingOps {
    blocking_ops: Vec<SuggestedPath>,
    /// Map of resolved funtion `def_id` with suggestion string after checking crate
    id_with_suggs: FxHashMap<DefId, Option<String>>,
    /// Tracking whether a body is async after entering it.
    body_asyncness: Vec<bool>,
}

impl UnnecessaryBlockingOps {
    pub(crate) fn new(blocking_ops: Vec<SuggestedPath>) -> Self {
        Self {
            blocking_ops,
            id_with_suggs: FxHashMap::default(),
            body_asyncness: vec![],
        }
    }
}

impl_lint_pass!(UnnecessaryBlockingOps => [UNNECESSARY_BLOCKING_OPS]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryBlockingOps {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let full_fn_list = self.blocking_ops.iter().map(|p| (p.path(), p.suggestion()));
        for (path_str, maybe_sugg_str) in full_fn_list {
            let path: Vec<&str> = path_str.split("::").collect();
            for did in def_path_def_ids(cx, &path) {
                self.id_with_suggs.insert(did, maybe_sugg_str.map(ToOwned::to_owned));
            }
        }
    }

    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
        if is_lint_allowed(cx, UNNECESSARY_BLOCKING_OPS, body.value.hir_id) {
            return;
        }
        self.body_asyncness.push(in_async_body(cx, body.id()));
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !in_external_macro(cx.sess(), expr.span)
            && matches!(self.body_asyncness.last(), Some(true))
            && let ExprKind::Call(call, _) = &expr.kind
            && let Some(call_did) = fn_def_id(cx, expr)
            && let Some(maybe_sugg) = self.id_with_suggs.get(&call_did)
        {
            span_lint_and_then(
                cx,
                UNNECESSARY_BLOCKING_OPS,
                call.span,
                "blocking function call detected in an async body",
                |diag| {
                    if let Some(sugg_fn_path) = maybe_sugg {
                        make_suggestion(diag, cx, expr, call.span, sugg_fn_path);
                    }
                },
            );
        }
    }

    fn check_body_post(&mut self, _: &LateContext<'tcx>, _: &'tcx Body<'tcx>) {
        self.body_asyncness.pop();
    }
}

fn make_suggestion(diag: &mut Diag<'_, ()>, cx: &LateContext<'_>, expr: &Expr<'_>, fn_span: Span, sugg_fn_path: &str) {
    // Suggestion should only be offered when user specified it in the configuration file,
    // so we only assume it can be fixed here if only the path could be found.
    let mut applicability = if def_path_def_ids(cx, &sugg_fn_path.split("::").collect::<Vec<_>>())
        .next()
        .is_some()
    {
        Applicability::MaybeIncorrect
    } else {
        Applicability::Unspecified
    };

    let args_span = expr.span.with_lo(fn_span.hi());
    let args_snippet = snippet_with_applicability(cx, args_span, "..", &mut applicability);
    let suggestion = format!("{sugg_fn_path}{args_snippet}.await");
    diag.span_suggestion(expr.span, "try using its async counterpart", suggestion, applicability);
}

/// Check whether a body is from an async function/closure.
fn in_async_body(cx: &LateContext<'_>, body_id: BodyId) -> bool {
    let parent_node = cx.tcx.parent_hir_node(body_id.hir_id);
    match parent_node {
        Node::Expr(expr) => matches!(
            expr.kind,
            ExprKind::Closure(Closure {
                kind: ClosureKind::Coroutine(CoroutineKind::Desugared(
                    CoroutineDesugaring::Async | CoroutineDesugaring::AsyncGen,
                    _
                )),
                ..
            })
        ),
        Node::Item(Item {
            kind: ItemKind::Fn(fn_sig, ..),
            ..
        })
        | Node::ImplItem(ImplItem {
            kind: ImplItemKind::Fn(fn_sig, _),
            ..
        })
        | Node::TraitItem(TraitItem {
            kind: TraitItemKind::Fn(fn_sig, _),
            ..
        }) => fn_sig.header.is_async(),
        _ => false,
    }
}
