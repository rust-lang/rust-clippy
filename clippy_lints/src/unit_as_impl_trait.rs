use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, ExprKind, FnDecl, FnRetTy, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::{BytePos, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for function whose return type is an `impl` trait
    /// implemented by `()`, and whose `()` return value is implicit.
    ///
    /// ### Why is this bad?
    /// Since the required trait is implemented by `()`, adding an extra `;`
    /// at the end of the function last expression will compile with no
    /// indication that the expected value may have been silently swallowed.
    ///
    /// ### Example
    /// ```no_run
    /// fn key() -> impl Eq {
    ///     [1, 10, 2, 0].sort_unstable()
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn key() -> impl Eq {
    ///     let mut arr = [1, 10, 2, 0];
    ///     arr.sort_unstable();
    ///     arr
    /// }
    /// ```
    /// or
    /// ```no_run
    /// fn key() -> impl Eq {
    ///     [1, 10, 2, 0].sort_unstable();
    ///     ()
    /// }
    /// ```
    /// if returning `()` is intentional.
    #[clippy::version = "1.90.0"]
    pub UNIT_AS_IMPL_TRAIT,
    suspicious,
    "`()` which implements the required trait might be returned unexpectedly"
}

declare_lint_pass!(UnitAsImplTrait => [UNIT_AS_IMPL_TRAIT]);

impl<'tcx> LateLintPass<'tcx> for UnitAsImplTrait {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        if !span.from_expansion()
            && let FnRetTy::Return(ty) = decl.output
            && let TyKind::OpaqueDef(_) = ty.kind
            && cx.typeck_results().expr_ty(body.value).is_unit()
            && let ExprKind::Block(block, None) = body.value.kind
            && block.expr.is_none_or(|e| !matches!(e.kind, ExprKind::Tup([])))
        {
            let block_end_span = block.span.with_hi(block.span.hi() - BytePos(1)).shrink_to_hi();

            span_lint_and_then(
                cx,
                UNIT_AS_IMPL_TRAIT,
                ty.span,
                "this function returns `()` which implements the required trait",
                |diag| {
                    if let Some(expr) = block.expr {
                        diag.span_note(expr.span, "this expression evaluates to `()`")
                            .span_help(
                                expr.span.shrink_to_hi(),
                                "consider being explicit, and terminate the body with `()`",
                            );
                    } else if let Some(last) = block.stmts.last() {
                        diag.span_note(last.span, "this statement evaluates to `()` because it ends with `;`")
                            .span_note(block.span, "therefore the function body evaluates to `()`")
                            .span_help(
                                block_end_span,
                                "if this is intentional, consider being explicit, and terminate the body with `()`",
                            );
                    } else {
                        diag.span_note(block.span, "the empty body evaluates to `()`")
                            .span_help(block_end_span, "consider being explicit and use `()` in the body");
                    }
                },
            );
        }
    }
}
