use clippy_utils::attrs::span_contains_cfg_or_cfg_attr;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{SpanExt as _, line_span, snippet_indent, walk_span_to_context};
use clippy_utils::ty::ty_satisfies_opaque_bounds;
use clippy_utils::{get_async_fn_body, is_block_like, is_from_proc_macro};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    self as hir, Block, Body, Expr, ExprKind, FnDecl, FnRetTy, GenericBound, OpaqueTy, OpaqueTyOrigin, Stmt, StmtKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext as _, declare_lint_pass};
use rustc_middle::ty::Ty;
use rustc_span::{BytePos, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions returning an `impl Trait` satisfied by `()`, which
    /// return `()` without an explicit `()` expression, that is when the body:
    /// - ends with an expression statement (terminated by `;`), and the type of
    ///   this expression also satisfies the bounds of the returned `impl Trait`;
    /// - ends with an expression of type `()`;
    /// - is empty.
    ///
    /// ### Why is this bad?
    /// Since `()` satisfies the bounds of the returned type, an extra `;` at the end
    /// of the function body, or a last expression which does not return the expected
    /// value, compiles without any error, and silently makes the function return `()`.
    ///
    /// ### Example
    /// ```no_run
    /// fn key(v: &[u32]) -> impl Ord {
    ///     v.iter().sum::<u32>();
    /// }
    ///
    /// fn sorted() -> impl Eq {
    ///     [1, 10, 2, 0].sort_unstable()
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn key(v: &[u32]) -> impl Ord {
    ///     v.iter().sum::<u32>()
    /// }
    ///
    /// fn sorted() -> impl Eq {
    ///     let mut a = [1, 10, 2, 0];
    ///     a.sort_unstable();
    ///     a
    /// }
    /// ```
    /// or, if returning `()` is intentional, make it explicit:
    /// ```no_run
    /// fn sorted() -> impl Eq {
    ///     [1, 10, 2, 0].sort_unstable();
    ///     ()
    /// }
    /// ```
    #[clippy::version = "1.102.0"]
    pub UNIT_AS_IMPL_TRAIT,
    suspicious,
    "implicitly returning `()` from a function returning an `impl Trait`"
}

declare_lint_pass!(UnitAsImplTrait => [UNIT_AS_IMPL_TRAIT]);

const UNIT_SATISFIES_BOUNDS: &str = "`()` satisfies the bounds of the returned type";
const MAKE_UNIT_EXPLICIT: &str = "if returning `()` is intentional, return it explicitly";

impl<'tcx> LateLintPass<'tcx> for UnitAsImplTrait {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        def_id: LocalDefId,
    ) {
        if !span.from_expansion()
            && let FnRetTy::Return(ret_ty) = decl.output
            && let Some((impl_ty, opaque, body_expr)) = user_impl_trait_and_body(cx, ret_ty, body)
            && let ExprKind::Block(block, None) = body_expr.kind
            && !block.span.from_expansion()
            && !span.in_external_macro(cx.sess().source_map())
        {
            let is_from_proc_macro =
                || is_from_proc_macro(cx, &(&kind, body, cx.tcx.local_def_id_to_hir_id(def_id), span));
            match (block.stmts.last(), block.expr) {
                (None, None) => {
                    if !span_contains_cfg_or_cfg_attr(cx, block.span) && !is_from_proc_macro() {
                        lint_empty_body(cx, impl_ty, block);
                    }
                },
                (_, Some(expr)) => {
                    if !matches!(expr.kind, ExprKind::Tup([]))
                        && cx.typeck_results().expr_ty(expr).is_unit()
                        && let Some(expr_span) = walk_span_to_context(expr.span, block.span.ctxt())
                        && !is_from_proc_macro()
                    {
                        lint_unit_tail_expr(cx, impl_ty, block, expr, expr_span);
                    }
                },
                (Some(last), None) => {
                    if let StmtKind::Semi(expr) = last.kind
                        && last.span.ctxt() == block.span.ctxt()
                        && expr.span.ctxt() == block.span.ctxt()
                        && let expr_ty = cx.typeck_results().expr_ty(expr)
                        && !expr_ty.is_unit()
                        && !expr_ty.is_never()
                        && ty_satisfies_opaque_bounds(cx, expr_ty, opaque.def_id.to_def_id())
                        && !is_from_proc_macro()
                    {
                        lint_trailing_semi(cx, impl_ty, last, expr, expr_ty);
                    }
                },
            }
        }
    }
}

fn lint_trailing_semi<'tcx>(
    cx: &LateContext<'tcx>,
    impl_ty: &hir::Ty<'_>,
    last: &Stmt<'_>,
    expr: &Expr<'_>,
    expr_ty: Ty<'tcx>,
) {
    span_lint_and_then(
        cx,
        UNIT_AS_IMPL_TRAIT,
        last.span,
        "this function returns `()` instead of the value of its last expression",
        |diag| {
            diag.span_note(
                impl_ty.span,
                format!("both `()` and `{expr_ty}` satisfy the bounds of the returned type"),
            );
            diag.span_suggestion_verbose(
                expr.span.shrink_to_hi().to(last.span.shrink_to_hi()),
                "if the value was meant to be returned, remove the trailing `;`",
                "",
                Applicability::MaybeIncorrect,
            );
            diag.help("if returning `()` is intentional, terminate the body with an explicit `()`");
        },
    );
}

fn lint_empty_body(cx: &LateContext<'_>, impl_ty: &hir::Ty<'_>, block: &Block<'_>) {
    span_lint_and_then(
        cx,
        UNIT_AS_IMPL_TRAIT,
        block.span,
        "this function implicitly returns `()` through its empty body",
        |diag| {
            diag.span_note(impl_ty.span, UNIT_SATISFIES_BOUNDS);
            let closing_brace = block.span.with_lo(block.span.hi() - BytePos(1)).shrink_to_lo();
            if block.span.check_text(cx, |src| {
                src.strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .is_some_and(|inner| inner.trim().is_empty())
            }) {
                diag.span_suggestion_verbose(block.span, MAKE_UNIT_EXPLICIT, "{ () }", Applicability::MaybeIncorrect);
            } else if let Some(Some(sugg)) = line_span(cx, closing_brace).with_source_text(cx, |indent| {
                // If the closing brace is alone on its line, insert `()` on its own line before it,
                // using the closing brace indentation.
                indent
                    .chars()
                    .all(char::is_whitespace)
                    .then(|| format!("    ()\n{indent}"))
            }) {
                diag.span_suggestion_verbose(closing_brace, MAKE_UNIT_EXPLICIT, sugg, Applicability::MaybeIncorrect);
            } else {
                diag.help(MAKE_UNIT_EXPLICIT);
            }
        },
    );
}

fn lint_unit_tail_expr(
    cx: &LateContext<'_>,
    impl_ty: &hir::Ty<'_>,
    block: &Block<'_>,
    expr: &Expr<'_>,
    expr_span: Span,
) {
    span_lint_and_then(
        cx,
        UNIT_AS_IMPL_TRAIT,
        expr_span,
        "this function implicitly returns `()` through its last expression",
        |diag| {
            diag.span_note(impl_ty.span, UNIT_SATISFIES_BOUNDS);
            // A block-like expression from the function body can be followed by `()` directly,
            // other expressions (including macro calls) must be terminated by `;` first.
            let sep = if expr.span.ctxt() == block.span.ctxt() && is_block_like(expr) {
                ""
            } else {
                ";"
            };
            let indent = snippet_indent(cx, expr_span).unwrap_or_default();
            diag.span_suggestion_verbose(
                expr_span.shrink_to_hi(),
                MAKE_UNIT_EXPLICIT,
                format!("{sep}\n{indent}()"),
                Applicability::MaybeIncorrect,
            );
        },
    );
}

/// If the user-written return type is an `impl Trait`, returns it along with the
/// corresponding opaque type and the expression representing the user-written body
/// of the function. For `async fn`, the user-written return type is the `Output` of
/// the `impl Future` produced by the desugaring, and the body is the one embedded
/// into the generated coroutine.
fn user_impl_trait_and_body<'tcx>(
    cx: &LateContext<'tcx>,
    ret_ty: &'tcx hir::Ty<'tcx>,
    body: &'tcx Body<'tcx>,
) -> Option<(&'tcx hir::Ty<'tcx>, &'tcx OpaqueTy<'tcx>, &'tcx Expr<'tcx>)> {
    let hir::TyKind::OpaqueDef(opaque) = ret_ty.kind else {
        return None;
    };
    match opaque.origin {
        OpaqueTyOrigin::FnReturn { .. } => Some((ret_ty, opaque, body.value)),
        OpaqueTyOrigin::AsyncFn { .. } => {
            if let [GenericBound::Trait(trait_ref), ..] = opaque.bounds
                && let Some(segment) = trait_ref.trait_ref.path.segments.last()
                && let Some(args) = segment.args
                && let [constraint] = args.constraints
                && let Some(output_ty) = constraint.ty()
                && let hir::TyKind::OpaqueDef(output_opaque) = output_ty.kind
                && let OpaqueTyOrigin::FnReturn { .. } = output_opaque.origin
                && let Some(body_expr) = get_async_fn_body(cx.tcx, body)
            {
                Some((output_ty, output_opaque, body_expr))
            } else {
                None
            }
        },
        OpaqueTyOrigin::TyAlias { .. } => None,
    }
}
