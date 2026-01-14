use crate::clippy_utils::res::MaybeDef;
use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::res::MaybeResPath;
use clippy_utils::source::snippet;
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_abi::Size;
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{FnKind, Visitor, walk_body, walk_expr};
use rustc_hir::{Body, Expr, ExprKind, FnDecl, HirIdSet, LangItem, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::Ty;
use rustc_session::impl_lint_pass;
use rustc_span::{DesugaringKind, Span};

declare_clippy_lint! {
    /// ### What it does
    /// It checks for the size of a `Future` created by `async fn` or `async {}`.
    ///
    /// ### Why is this bad?
    /// Due to the current [unideal implementation](https://github.com/rust-lang/rust/issues/69826) of `Coroutine`,
    /// large size of a `Future` may cause stack overflows or be inefficient.
    ///
    /// ### Example
    /// ```no_run
    /// async fn large_future(_x: [u8; 16 * 1024]) {}
    ///
    /// pub async fn trigger() {
    ///     large_future([0u8; 16 * 1024]).await;
    /// }
    /// ```
    ///
    /// `Box::pin` the big future or the captured elements inside the future instead.
    ///
    /// ```no_run
    /// async fn large_future(_x: [u8; 16 * 1024]) {}
    ///
    /// pub async fn trigger() {
    ///     Box::pin(large_future([0u8; 16 * 1024])).await;
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub LARGE_FUTURES,
    pedantic,
    "large future may lead to unexpected stack overflows"
}

pub struct LargeFuture {
    future_size_threshold: u64,
    visited: HirIdSet,
}

impl LargeFuture {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            future_size_threshold: conf.future_size_threshold,
            visited: HirIdSet::default(),
        }
    }
}

impl_lint_pass!(LargeFuture => [LARGE_FUTURES]);

struct AsyncFnVisitor<'a, 'tcx> {
    parent: &'a mut LargeFuture,
    cx: &'a LateContext<'tcx>,
    /// Depth of the visitor with value `0` denoting the top-level expression.
    depth: usize,
    /// Whether a clippy suggestion was emitted for deeper levels of expressions.
    emitted: bool,
}

impl<'tcx> Visitor<'tcx> for AsyncFnVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // For each expression, try to find the 'deepest' occurance of a too large a future.
        // Chances are that subsequent less deep expression contain that too large a future,
        // and hence it is redudant to also emit a suggestion for these expressions.
        // Expressions on the same depth level can however emit a suggestion too.

        if !self.parent.visited.insert(expr.hir_id) {
            // This expression was already visited once.
            return;
        }

        if let ExprKind::Call(method, _) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, seg)) = method.kind
            && ty.basic_res().is_lang_item(self.cx, LangItem::OwnedBox)
            && seg.ident.name == sym::pin
        {
            // When the current expression is already a `Box::pin(...)` call,
            // our suggestion has been applied. Do not recurse deeper.
            return;
        }

        self.depth += 1;
        let old_emitted = self.emitted;
        self.emitted = false; // Reset emitted for deeper level, but recover it later.
        walk_expr(self, expr);
        self.depth -= 1;

        if !self.emitted
            && !expr.span.is_desugaring(DesugaringKind::Await)
            && let Some(ty) = self.cx.typeck_results().expr_ty_opt(expr)
            && let Some(size) = type_is_large_future(self.cx, ty, self.parent.future_size_threshold)
        {
            if self.depth == 0 {
                // Special lint for top level fn bodies.
                span_lint_hir_and_then(
                    self.cx,
                    LARGE_FUTURES,
                    expr.hir_id,
                    expr.span,
                    format!("definition for a large future with a size of {} bytes", size.bytes()),
                    |db| {
                        db.span_help(expr.span, "consider `Box::pin` when constructing it or reducing direct ownership of the items in scope and use references instead where possible");
                    },
                );
            } else {
                // Expressions within a fn body.
                span_lint_hir_and_then(
                    self.cx,
                    LARGE_FUTURES,
                    expr.hir_id,
                    expr.span,
                    format!("usage of large future with a size of {} bytes", size.bytes()),
                    |db| {
                        db.span_suggestion(
                            expr.span,
                            "consider `Box::pin` on it or reducing direct ownership of the items in scope and use references instead where possible",
                            format!("Box::pin({})", snippet(self.cx, expr.span, "..")),
                            Applicability::Unspecified,
                        );
                    },
                );
            }

            self.emitted = true;
        }

        self.emitted |= old_emitted;
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}

fn type_is_large_future<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, future_size_threshold: u64) -> Option<Size> {
    if let Some(future_trait_def_id) = cx.tcx.lang_items().future_trait()
        && implements_trait(cx, ty, future_trait_def_id, &[])
        && let Ok(layout) = cx.tcx.layout_of(cx.typing_env().as_query_input(ty))
        && let size = layout.layout.size()
        && size >= Size::from_bytes(future_size_threshold)
    {
        Some(size)
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for LargeFuture {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: LocalDefId,
    ) {
        let mut visitor = AsyncFnVisitor {
            parent: self,
            cx,
            depth: 0,
            emitted: false,
        };
        walk_body(&mut visitor, body);
    }
}
