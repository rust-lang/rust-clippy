use std::ops::Not;

use clippy_utils::as_some_expr;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef, MaybeQPath, MaybeResPath};

use clippy_utils::sugg::Sugg;
use rustc_hir::LangItem::OptionNone;
use rustc_hir::{Arm, Expr, ExprKind, HirId, Pat, PatKind};
use rustc_lint::LateContext;
use rustc_span::{SyntaxContext, sym};

use super::MANUAL_FILTER;
use super::manual_utils::{SomeExpr, check_with};

// Function called on the <expr> of `[&+]Some((ref | ref mut) x) => <expr>`
// Need to check if it's of the form `<expr>=if <cond> {<then_expr>} else {<else_expr>}`
// AND that only one `then/else_expr` resolves to `Some(x)` while the other resolves to `None`
// return the `cond` expression if so.
fn get_cond_expr<'a, 'tcx>(
    cx: &LateContext<'tcx>,
    pat: &Pat<'_>,
    expr: &'tcx Expr<'_>,
    ctxt: SyntaxContext,
) -> Option<SomeExpr<'a, 'tcx>> {
    if let ExprKind::If(cond, then_expr, Some(else_expr)) = expr.kind
        && let PatKind::Binding(_, target, ..) = pat.kind
        // check that one expr resolves to `Some(x)`, the other to `None`
        && let need_neg = (is_none_expr(cx, then_expr) && is_some_expr(cx, target, ctxt, else_expr))
        && (need_neg || is_some_expr(cx, target, ctxt, then_expr) && is_none_expr(cx, else_expr))
    {
        return Some(SomeExpr {
            expr: cond.peel_drop_temps(),
            extra_fn: need_neg.then_some(&Sugg::not), /* need to negate the condition if the `then_expr` resolves to
                                                       * `None` */
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
fn is_some_expr(cx: &LateContext<'_>, target: HirId, ctxt: SyntaxContext, expr: &Expr<'_>) -> bool {
    if let Some(inner_expr) = peels_blocks_incl_unsafe_opt(expr)
        // there can be not statements in the block as they would be removed when switching to `.filter`
        && let Some(arg) = as_some_expr(cx, inner_expr)
    {
        return ctxt == expr.span.ctxt() && arg.res_local_id() == Some(target);
    }
    false
}

fn is_none_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(inner_expr) = peels_blocks_incl_unsafe_opt(expr) {
        return inner_expr.res(cx).ctor_parent(cx).is_lang_item(cx, OptionNone);
    }
    false
}

// given the closure: `|<pattern>| <expr>`
// returns `|&<pattern>| <expr>`
fn add_ampersand_if_copy(body_str: String, has_copy_trait: bool) -> String {
    if has_copy_trait {
        let mut with_ampersand = body_str;
        with_ampersand.insert(1, '&');
        with_ampersand
    } else {
        body_str
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
        let body_str = add_ampersand_if_copy(sugg_info.body_str, sugg_info.scrutinee_impl_copy);
        span_lint_and_sugg(
            cx,
            MANUAL_FILTER,
            expr.span,
            "manual implementation of `Option::filter`",
            "try",
            if sugg_info.needs_brackets {
                format!(
                    "{{ {}{}.filter({body_str}) }}",
                    sugg_info.scrutinee_str, sugg_info.as_ref_str
                )
            } else {
                format!("{}{}.filter({body_str})", sugg_info.scrutinee_str, sugg_info.as_ref_str)
            },
            sugg_info.app,
        );
    }
}
