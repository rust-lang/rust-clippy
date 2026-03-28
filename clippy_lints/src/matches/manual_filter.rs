use std::ops::Not;

use clippy_utils::as_some_expr;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeQPath, MaybeResPath};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_copy;
use rustc_errors::Applicability;
use rustc_hir::LangItem::OptionNone;
use rustc_hir::{Arm, Expr, ExprKind, Pat, PatKind};
use rustc_lint::LateContext;
use rustc_span::{Span, SyntaxContext, sym};

use super::MANUAL_FILTER;
use super::manual_utils::{SomeExpr, check_with, get_some_expr_with_block_info};

struct SuggExtraInfo {
    use_filter: bool,
}

// Function called on the <expr> of `[&+]Some((ref | ref mut) x) => <expr>`
// Need to check if it's of the form `<expr>=if <cond> {<then_expr>} else {<else_expr>}`
// AND that only one `then/else_expr` resolves to `Some(x)` while the other resolves to `None`
// return the `cond` expression if so.
fn get_cond_expr<'a, 'tcx>(
    cx: &LateContext<'tcx>,
    pat: &Pat<'_>,
    expr: &'tcx Expr<'_>,
    ctxt: SyntaxContext,
) -> Option<SomeExpr<'a, 'tcx, SuggExtraInfo>> {
    if let ExprKind::If(cond, then_expr, Some(else_expr)) = expr.kind
        && let PatKind::Binding(_, target, ..) = pat.kind
        // check that one expr resolves to `Some(x)`, the other to `None`
        && let Some((need_neg, inner)) =
            is_none_expr(cx, then_expr)
            .then(
                || is_some_expr(cx, ctxt, else_expr).map(|e| (true, e))).flatten()
            .or_else(
                || is_none_expr(cx, else_expr).then(|| is_some_expr(cx, ctxt, then_expr).map(|e| (false, e))).flatten())
    {
        return Some(if inner.res_local_id() == Some(target) {
            SomeExpr {
                expr: cond.peel_drop_temps(),
                extra_fn: need_neg.then_some(&Sugg::not), /* need to negate the condition if the `then_expr` resolves
                                                           * to `None` */
                extra_info: SuggExtraInfo { use_filter: true },
            }
        } else {
            SomeExpr {
                expr,
                extra_fn: None,
                extra_info: SuggExtraInfo { use_filter: false },
            }
        });
    }

    None
}

fn peels_blocks_incl_unsafe_opt<'a>(expr: &'a Expr<'a>) -> Option<&'a Expr<'a>> {
    // we don't want to use `peel_blocks` here because we don't care if the block is unsafe, it's
    // checked by `contains_unsafe_block`
    if let ExprKind::Block(block, None) = expr.kind
        && block.stmts.is_empty()
    {
        return block.expr;
    }
    None
}

/// Checks whether <expr> resolves to `Some(target)`
// NOTE: called for each <expr> expression:
// Some(x) => if <cond> {
//    <expr>
// } else {
//    <expr>
// }
fn is_some_expr<'tcx>(cx: &LateContext<'tcx>, ctxt: SyntaxContext, expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let Some(inner_expr) = peels_blocks_incl_unsafe_opt(expr)
        // there can be not statements in the block as they would be removed when switching to `.filter`
        && let Some(arg) = as_some_expr(cx, inner_expr)
        && ctxt == expr.span.ctxt()
    {
        return Some(arg);
    }
    None
}

fn is_none_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(inner_expr) = peels_blocks_incl_unsafe_opt(expr) {
        return inner_expr.res(cx).ctor_parent(cx).is_lang_item(cx, OptionNone);
    }
    false
}

// given the closure: `|<pattern>| <expr>`
// returns `|&<pattern>| <expr>`
fn add_ampersand_if_copy(mut body_str: String, has_copy_trait: bool) -> String {
    if has_copy_trait {
        body_str.insert(1, '&');
    }
    body_str
}

/// Checks for the following pattern:
/// `opt.and_then(|x| if /* predicate on x */ { Some(x) } else { None })`
/// and suggests replacing with:
/// `opt.filter(|&x| /* predicate on x */ )`
pub(crate) fn check_and_then_method<'tcx>(
    cx: &LateContext<'tcx>,
    scrutinee: &'tcx Expr<'_>,
    arg: &'tcx Expr<'_>,
    call_span: Span,
    expr: &'tcx Expr<'_>,
) {
    let ty = cx.typeck_results().expr_ty(scrutinee);
    if ty.is_diag_item(cx, sym::Option)
        && let ExprKind::Closure(closure) = arg.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let Some(fn_arg_span) = closure.fn_arg_span
        && let [param] = body.params
        && let expr_span_ctxt = expr.span.ctxt()
        && let Some(some_expr) =
            get_some_expr_with_block_info(cx, param.pat, body.value, expr_span_ctxt, &get_cond_expr)
        && some_expr.some_expr.extra_info.use_filter
    {
        span_lint_and_then(
            cx,
            MANUAL_FILTER,
            call_span,
            "manual implementation of `Option::filter`",
            |diag| {
                let mut applicability = Applicability::MachineApplicable;
                let cond_snip = some_expr.to_snippet_with_context(cx, expr_span_ctxt, &mut applicability);

                let (prefix_snip, _) = snippet_with_context(
                    cx,
                    closure.fn_decl_span.until(fn_arg_span),
                    expr_span_ctxt,
                    "..",
                    &mut applicability,
                );
                let (param_snip, _) =
                    snippet_with_context(cx, param.pat.span, expr_span_ctxt, "..", &mut applicability);
                diag.span_suggestion(
                    call_span,
                    "try",
                    format!(
                        "filter({prefix_snip}|{}{param_snip}| {cond_snip})",
                        if is_copy(cx, ty) { "&" } else { "" }
                    ),
                    applicability,
                );
            },
        );
    }
}

pub(super) fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    scrutinee: &'tcx Expr<'_>,
    arms: &'tcx [Arm<'_>],
    expr: &'tcx Expr<'_>,
) {
    let ty = cx.typeck_results().expr_ty(expr);
    if ty.is_diag_item(cx, sym::Option)
        && let [first_arm, second_arm] = arms
        && first_arm.guard.is_none()
        && second_arm.guard.is_none()
    {
        check(
            cx,
            expr,
            scrutinee,
            first_arm.pat,
            first_arm.body,
            Some(second_arm.pat),
            second_arm.body,
        );
    }
}

pub(super) fn check_if_let<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    let_pat: &'tcx Pat<'_>,
    let_expr: &'tcx Expr<'_>,
    then_expr: &'tcx Expr<'_>,
    else_expr: &'tcx Expr<'_>,
) {
    check(cx, expr, let_expr, let_pat, then_expr, None, else_expr);
}

fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    scrutinee: &'tcx Expr<'_>,
    then_pat: &'tcx Pat<'_>,
    then_body: &'tcx Expr<'_>,
    else_pat: Option<&'tcx Pat<'_>>,
    else_body: &'tcx Expr<'_>,
) {
    if let Some(sugg_info) = check_with(
        cx,
        expr,
        scrutinee,
        then_pat,
        then_body,
        else_pat,
        else_body,
        &get_cond_expr,
    ) {
        let method = if sugg_info.extra_info.use_filter {
            "filter"
        } else {
            "and_then"
        };
        span_lint_and_then(
            cx,
            MANUAL_FILTER,
            expr.span,
            format!("manual implementation of `Option::{method}`"),
            |diag| {
                let mut body_str = sugg_info.body_str;
                if sugg_info.extra_info.use_filter {
                    let scrutinee_ty = cx.typeck_results().expr_ty(scrutinee);
                    body_str = add_ampersand_if_copy(body_str, is_copy(cx, scrutinee_ty)); // relies on the fact that Option<T>: Copy where T: copy
                }

                diag.span_suggestion(
                    expr.span,
                    "try",
                    if sugg_info.needs_brackets {
                        format!(
                            "{{ {}{}.{method}({body_str}) }}",
                            sugg_info.scrutinee_str, sugg_info.as_ref_str
                        )
                    } else {
                        format!(
                            "{}{}.{method}({body_str})",
                            sugg_info.scrutinee_str, sugg_info.as_ref_str
                        )
                    },
                    sugg_info.app,
                );
            },
        );
    }
}
