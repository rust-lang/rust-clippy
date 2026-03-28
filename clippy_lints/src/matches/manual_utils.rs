use crate::map_unit_fn::OPTION_MAP_UNIT_FN;
use crate::matches::MATCH_AS_REF;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::source::{snippet_with_applicability, snippet_with_context};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{is_unsafe_fn, peel_and_count_ty_refs};
use clippy_utils::{
    CaptureKind, as_some_pattern, can_move_expr_to_closure, expr_requires_coercion, is_else_clause, is_lint_allowed,
    is_none_expr, is_none_pattern, peel_blocks, peel_hir_expr_refs, peel_hir_expr_while,
};
use rustc_ast::util::parser::ExprPrecedence;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{
    BindingMode, Block, BlockCheckMode, Expr, ExprKind, HirId, Mutability, Pat, PatKind, Path, QPath, UnsafeSource,
};
use rustc_lint::LateContext;
use rustc_span::{Span, SyntaxContext, sym};

type GetSomeExprFn<'a, 'tcx, T> =
    dyn Fn(&LateContext<'tcx>, &'tcx Pat<'_>, &'tcx Expr<'_>, SyntaxContext) -> Option<SomeExpr<'a, 'tcx, T>> + 'a;

#[expect(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines)]
pub(super) fn check_with<'a, 'tcx, T>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    scrutinee: &'tcx Expr<'_>,
    then_pat: &'tcx Pat<'_>,
    then_body: &'tcx Expr<'_>,
    else_pat: Option<&'tcx Pat<'_>>,
    else_body: &'tcx Expr<'_>,
    get_some_expr_fn: &'a GetSomeExprFn<'a, 'tcx, T>,
) -> Option<SuggInfo<'tcx, T>> {
    let (scrutinee_ty, ty_ref_count, ty_mutability) = peel_and_count_ty_refs(cx.typeck_results().expr_ty(scrutinee));
    let ty_mutability = ty_mutability.unwrap_or(Mutability::Mut);

    if !(scrutinee_ty.is_diag_item(cx, sym::Option) && cx.typeck_results().expr_ty(expr).is_diag_item(cx, sym::Option))
    {
        return None;
    }

    let expr_ctxt = expr.span.ctxt();
    let (some_expr, some_pat, pat_ref_count, is_wild_none) = match (
        try_parse_pattern(cx, then_pat, expr_ctxt),
        else_pat.map_or(Some(OptionPat::Wild), |p| try_parse_pattern(cx, p, expr_ctxt)),
    ) {
        (Some(OptionPat::Wild), Some(OptionPat::Some { pattern, ref_count })) if is_none_arm_body(cx, then_body) => {
            (else_body, pattern, ref_count, true)
        },
        (Some(OptionPat::None), Some(OptionPat::Some { pattern, ref_count })) if is_none_arm_body(cx, then_body) => {
            (else_body, pattern, ref_count, false)
        },
        (Some(OptionPat::Some { pattern, ref_count }), Some(OptionPat::Wild)) if is_none_arm_body(cx, else_body) => {
            (then_body, pattern, ref_count, true)
        },
        (Some(OptionPat::Some { pattern, ref_count }), Some(OptionPat::None)) if is_none_arm_body(cx, else_body) => {
            (then_body, pattern, ref_count, false)
        },
        _ => return None,
    };

    // Top level or patterns aren't allowed in closures.
    if matches!(some_pat.kind, PatKind::Or(_)) {
        return None;
    }

    let some_expr_within_block = get_some_expr_with_block_info(cx, some_pat, some_expr, expr_ctxt, get_some_expr_fn)?;

    // These two lints will go back and forth with each other.
    if cx.typeck_results().expr_ty(some_expr_within_block.some_expr.expr) == cx.tcx.types.unit
        && !is_lint_allowed(cx, OPTION_MAP_UNIT_FN, expr.hir_id)
    {
        return None;
    }

    // `map` won't perform any adjustments.
    if expr_requires_coercion(cx, expr) {
        return None;
    }

    // Determine which binding mode to use.
    let explicit_ref = some_pat.contains_explicit_ref_binding();
    let binding_ref = explicit_ref.or_else(|| (ty_ref_count != pat_ref_count).then_some(ty_mutability));

    let as_ref_str = match binding_ref {
        Some(Mutability::Mut) => ".as_mut()",
        Some(Mutability::Not) => ".as_ref()",
        None => "",
    };

    let captures = can_move_expr_to_closure(
        cx,
        some_expr_within_block
            .block_info
            .map_or(some_expr_within_block.some_expr.expr, |b| b.outermost_block),
    )?;
    // Check if captures the closure will need conflict with borrows made in the scrutinee.
    // TODO: check all the references made in the scrutinee expression. This will require interacting
    // with the borrow checker. Currently only `<local>[.<field>]*` is checked for.
    if let Some(binding_ref_mutability) = binding_ref {
        let e = peel_hir_expr_while(scrutinee, |e| match e.kind {
            ExprKind::Field(e, _) | ExprKind::AddrOf(_, _, e) => Some(e),
            _ => None,
        });
        if let ExprKind::Path(QPath::Resolved(None, Path { res: Res::Local(l), .. })) = e.kind {
            match captures.get(l) {
                Some(CaptureKind::Value | CaptureKind::Use | CaptureKind::Ref(Mutability::Mut)) => return None,
                Some(CaptureKind::Ref(Mutability::Not)) if binding_ref_mutability == Mutability::Mut => {
                    return None;
                },
                Some(CaptureKind::Ref(Mutability::Not)) | None => (),
            }
        }
    }

    let mut app = Applicability::MachineApplicable;

    // Remove address-of expressions from the scrutinee. Either `as_ref` will be called, or
    // it's being passed by value.
    let scrutinee = peel_hir_expr_refs(scrutinee).0;
    let (scrutinee_str, _) = snippet_with_context(cx, scrutinee.span, expr_ctxt, "..", &mut app);
    let scrutinee_str = if scrutinee.span.eq_ctxt(expr.span) && cx.precedence(scrutinee) < ExprPrecedence::Unambiguous {
        format!("({scrutinee_str})")
    } else {
        scrutinee_str.into()
    };

    let closure_body = some_expr_within_block.to_snippet_with_context(cx, expr_ctxt, &mut app);

    let body_str = if let PatKind::Binding(annotation, id, some_binding, None) = some_pat.kind {
        if some_expr_within_block.block_info.is_none()
            && let Some(func) = can_pass_as_func(cx, id, some_expr_within_block.some_expr.expr)
            && func.span.eq_ctxt(some_expr_within_block.some_expr.expr.span)
        {
            snippet_with_applicability(cx, func.span, "..", &mut app).into_owned()
        } else {
            if some_expr_within_block.some_expr.expr.res_local_id() == Some(id)
                && !is_lint_allowed(cx, MATCH_AS_REF, expr.hir_id)
                && binding_ref.is_some()
            {
                return None;
            }

            // `ref` and `ref mut` annotations were handled earlier.
            let annotation = if matches!(annotation, BindingMode::MUT) {
                "mut "
            } else {
                ""
            };

            format!("|{annotation}{some_binding}| {closure_body}")
        }
    } else if !is_wild_none && explicit_ref.is_none() {
        // TODO: handle explicit reference annotations.
        let pat_snip = snippet_with_context(cx, some_pat.span, expr_ctxt, "..", &mut app).0;
        format!("|{pat_snip}| {closure_body}")
    } else {
        // Refutable bindings and mixed reference annotations can't be handled by `map`.
        return None;
    };

    Some(SuggInfo {
        needs_brackets: else_pat.is_none() && is_else_clause(cx.tcx, expr),
        scrutinee_str,
        as_ref_str,
        body_str,
        app,
        extra_info: some_expr_within_block.some_expr.extra_info,
    })
}

pub struct SuggInfo<'a, T> {
    pub needs_brackets: bool,
    pub scrutinee_str: String,
    pub as_ref_str: &'a str,
    pub body_str: String,
    pub app: Applicability,
    pub extra_info: T,
}

// Checks whether the expression could be passed as a function, or whether a closure is needed.
// Returns the function to be passed to `map` if it exists.
fn can_pass_as_func<'tcx>(cx: &LateContext<'tcx>, binding: HirId, expr: &'tcx Expr<'_>) -> Option<&'tcx Expr<'tcx>> {
    match expr.kind {
        ExprKind::Call(func, [arg])
            if arg.res_local_id() == Some(binding)
                && cx.typeck_results().expr_adjustments(arg).is_empty()
                && !is_unsafe_fn(cx, cx.typeck_results().expr_ty(func).peel_refs()) =>
        {
            Some(func)
        },
        _ => None,
    }
}

#[derive(Debug)]
pub(super) enum OptionPat<'a> {
    Wild,
    None,
    Some {
        // The pattern contained in the `Some` tuple.
        pattern: &'a Pat<'a>,
        // The number of references before the `Some` tuple.
        // e.g. `&&Some(_)` has a ref count of 2.
        ref_count: usize,
    },
}

type SomeExprExtraFn<'a, 'tcx> = dyn Fn(Sugg<'tcx>) -> Sugg<'tcx> + 'a;

pub(super) struct SomeExpr<'a, 'tcx, T = ()> {
    pub expr: &'tcx Expr<'tcx>,
    pub extra_fn: Option<&'a SomeExprExtraFn<'a, 'tcx>>,
    pub extra_info: T,
}

struct SomeExprWithinBlock<'a, 'tcx, T> {
    some_expr: SomeExpr<'a, 'tcx, T>,
    block_info: Option<BlockInfo<'tcx>>,
}

/// If the target expression is from a block where there are statements preceding it, the
/// outer-most of such block, plus the span before and after the expression is used so that the
/// entire block can be included in the suggestion.
///
/// E.g. in `Some(x) => { println!("foo"); Some(x) }`, the expression `Some(x)` is from the
/// block `{ println!("foo"); Some(x) }`.
#[derive(Clone, Copy, Debug)]
struct BlockInfo<'tcx> {
    outermost_block: &'tcx Expr<'tcx>,
    span_before: Span,
    span_after: Span,
}

impl<'tcx, T> SomeExprWithinBlock<'_, 'tcx, T> {
    fn to_snippet_with_context(&self, cx: &LateContext<'tcx>, ctxt: SyntaxContext, app: &mut Applicability) -> String {
        let mut sugg = Sugg::hir_with_context(cx, self.some_expr.expr, ctxt, "..", app);
        if let Some(extra_fn) = self.some_expr.extra_fn {
            sugg = extra_fn(sugg);
        }

        let Some(block_info) = &self.block_info else {
            return sugg.to_string();
        };
        let (before, _) = snippet_with_context(cx, block_info.span_before, ctxt, "..", app);
        let (after, _) = snippet_with_context(cx, block_info.span_after, ctxt, "..", app);
        format!("{before}{sugg}{after}")
    }
}

fn get_some_expr_with_block_info<'a, 'tcx, T>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    expr: &'tcx Expr<'_>,
    ctxt: SyntaxContext,
    get_some_expr_fn: &'a GetSomeExprFn<'a, 'tcx, T>,
) -> Option<SomeExprWithinBlock<'a, 'tcx, T>> {
    match expr.kind {
        ExprKind::Block(
            Block {
                stmts,
                expr: Some(inner_expr),
                rules,
                ..
            },
            _,
        ) if let Some(mut some_expr) = get_some_expr_with_block_info(cx, pat, inner_expr, ctxt, get_some_expr_fn) => {
            if stmts.is_empty() && !matches!(rules, BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided)) {
                return Some(some_expr);
            }

            if let Some(block_info) = &mut some_expr.block_info {
                block_info.outermost_block = expr;
                block_info.span_before = block_info.span_before.with_lo(expr.span.lo());
                block_info.span_after = block_info.span_after.with_hi(expr.span.hi());
            } else {
                some_expr.block_info = Some(BlockInfo {
                    outermost_block: expr,
                    span_before: expr.span.until(inner_expr.span),
                    span_after: inner_expr.span.between(expr.span.shrink_to_hi()),
                });
            }
            Some(some_expr)
        },
        _ => {
            let some_expr = get_some_expr_fn(cx, pat, expr, ctxt)?;
            Some(SomeExprWithinBlock {
                some_expr,
                block_info: None,
            })
        },
    }
}

// Try to parse into a recognized `Option` pattern.
// i.e. `_`, `None`, `Some(..)`, or a reference to any of those.
pub(super) fn try_parse_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    ctxt: SyntaxContext,
) -> Option<OptionPat<'tcx>> {
    fn f<'tcx>(
        cx: &LateContext<'tcx>,
        pat: &'tcx Pat<'_>,
        ref_count: usize,
        ctxt: SyntaxContext,
    ) -> Option<OptionPat<'tcx>> {
        match pat.kind {
            PatKind::Wild => Some(OptionPat::Wild),
            PatKind::Ref(pat, _, _) => f(cx, pat, ref_count + 1, ctxt),
            _ if is_none_pattern(cx, pat) => Some(OptionPat::None),
            _ if let Some([pattern]) = as_some_pattern(cx, pat)
                && pat.span.ctxt() == ctxt =>
            {
                Some(OptionPat::Some { pattern, ref_count })
            },
            _ => None,
        }
    }
    f(cx, pat, 0, ctxt)
}

/// Checks for the `None` value, possibly in a block.
fn is_none_arm_body(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_none_expr(cx, peel_blocks(expr))
}
