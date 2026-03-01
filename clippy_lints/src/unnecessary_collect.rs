use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sym;
use clippy_utils::ty::{get_iterator_item_ty, has_non_owning_mutable_access, implements_trait};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;

use rustc_hir::{Block, Body, ExprKind, FnDecl, Stmt, StmtKind};
use rustc_lint::LateLintPass;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions returning `Vec<T>` where the `T` is produced from a `_.collect::<Vec<T>>()`
    ///
    /// ### Why is this bad?
    ///
    /// This might not be necessary, and its often more performant to return the iterator, giving the caller the option to use it as they see fit.
    /// Its often silly to have the equivalent of `iterator.collect::<Vec<_>>().into_iter()` in your code.
    ///
    /// ### Potential Issues
    ///
    /// In some cases, there may be some lifetimes attached to your iterator that make it difficult, or even impossible, to avoid a collection.
    /// If all uses of this function `collect` anyways, one ought simply `#[expect(clippy::unnecessary_collect)]`.
    ///
    /// ### Example
    /// ```no_run
    /// fn squares() -> Vec<(u8, u8)> {
    ///   (0..8u8)
    ///     .flat_map(|x| (0..8).map(move |y| (x, y)))
    ///     .collect::<Vec<_>>()
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn squares() -> impl Iterator<Item = (u8, u8)> {
    ///   (0..8u8).flat_map(|x| (0..8).map(move |y| (x, y)))
    /// }
    /// ```
    #[clippy::version = "1.92.0"]
    pub UNNECESSARY_COLLECT,
    nursery,
    "checks for functions returning vecs produced from vec::from_iter"
}

pub struct UnnecessaryCollect {
    avoid_breaking_exported_api: bool,
}

impl UnnecessaryCollect {
    pub fn new(
        &Conf {
            avoid_breaking_exported_api,
            ..
        }: &'static Conf,
    ) -> Self {
        Self {
            avoid_breaking_exported_api,
        }
    }
}

impl_lint_pass!(UnnecessaryCollect => [UNNECESSARY_COLLECT]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryCollect {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _: Span,
        def_id: LocalDefId,
    ) {
        if let FnKind::ItemFn(..) | FnKind::Method(..) = kind
            && let ExprKind::Block(
                Block { expr: Some(expr), .. }
                | Block { stmts: [.., Stmt { kind: StmtKind::Expr(expr), .. }], .. },
                _,
            ) = &body.value.kind
            && let expr = expr.peel_drop_temps()
            && let ExprKind::MethodCall(path, iter_expr, args, call_span) = expr.kind
            && !(self.avoid_breaking_exported_api && cx.effective_visibilities.is_exported(def_id))
            && !args.iter().any(|e| e.span.from_expansion())
            && path.ident.name == sym::collect
            && let iter_ty = cx.typeck_results().expr_ty(iter_expr)
            && !has_non_owning_mutable_access(cx, iter_ty)
            && let Some(iterator_trait_id) = cx.tcx.get_diagnostic_item(sym::Iterator)
            && implements_trait(cx, iter_ty, iterator_trait_id, &[])
            && let Some(item) = get_iterator_item_ty(cx, iter_ty)
            && let vec_ty = cx.typeck_results().expr_ty(expr)
            && let ty::Adt(v, args) = vec_ty.kind()
            && cx.tcx.is_diagnostic_item(sym::Vec, v.did())
            // make sure we're collecting to just a vec, and not a vec<result> String or other such tomfoolery
            && args.first().and_then(|x| x.as_type()) == Some(item)
        {
            span_lint_and_then(
                cx,
                UNNECESSARY_COLLECT,
                call_span,
                "this collection may not be necessary. return the iterator itself".to_string(),
                |diag| {
                    diag.span_label(call_span, "remove this collect");
                    diag.span_suggestion(
                        decl.output.span(),
                        "consider",
                        format!("impl Iterator<Item = {item}>"),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }
}
