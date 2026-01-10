//! This module defines both `unnecessary_map_or` (`.map_or(false, |n| n == 5)` -> `== Some(5)`)
//! and `needless_is_variant_and` (`.is_some_and(|n| n == 5)` -> `== Some(5)`
//!
//! The reason we can't remove the former in favor of `manual_is_variant_and` +
//! `needless_is_variant_and` is that the "is variant and" methods have high MSRVs, which would
//! unnecessarily stop the composed transformation
//! ```txt .map_or(false, |n| n == 5)
//! -> .is_some_and(|n| n == 5)
//! -> == Some(5)
//! ```
//! from happening on older versions of Rust.

use std::borrow::Cow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::eager_or_lazy::switch_to_eager_eval;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::sugg::{Sugg, make_binop};
use clippy_utils::ty::{implements_trait, is_copy};
use clippy_utils::visitors::is_local_used;
use clippy_utils::{get_parent_expr, is_from_proc_macro};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, PatKind};
use rustc_lint::{LateContext, Lint};
use rustc_span::sym;

use super::{NEEDLESS_IS_VARIANT_AND, UNNECESSARY_MAP_OR};

/// Checks that `map_or` is one of the following:
/// - `.map_or(false, |x| x == y)`
/// - `.map_or(false, |x| y == x)` - swapped comparison
/// - `.map_or(true, |x| x != y)`
/// - `.map_or(true, |x| y != x)` - swapped comparison
pub(super) fn check_map_or<'a>(
    cx: &LateContext<'a>,
    expr: &Expr<'a>,
    recv: &Expr<'_>,
    def: &Expr<'_>,
    map: &'a Expr<'a>,
) -> bool {
    if let ExprKind::Lit(def_kind) = def.kind
        && let LitKind::Bool(def_bool) = def_kind.node
    {
        return check_inner(cx, expr, recv, "map_or", map, def_bool, UNNECESSARY_MAP_OR);
    }
    false
}

/// Checks for `.is_some_and(|x| x == y)` or `.is_some_and(|x| y == x)`
pub(super) fn check_is_some_and<'a>(cx: &LateContext<'a>, expr: &Expr<'a>, recv: &Expr<'_>, map: &'a Expr<'a>) {
    let def_bool = false; // `is_some_and` is equiv to `map_or(false, `
    check_inner(cx, expr, recv, "is_some_and", map, def_bool, NEEDLESS_IS_VARIANT_AND);
}

/// Checks for `.is_none_or(|x| x != y)` or `.is_none_or(|x| y != x)`
pub(super) fn check_is_none_or<'a>(cx: &LateContext<'a>, expr: &Expr<'a>, recv: &Expr<'_>, map: &'a Expr<'a>) {
    let def_bool = true; // `is_none_or` is equiv to `map_or(true, `
    check_inner(cx, expr, recv, "is_none_or", map, def_bool, NEEDLESS_IS_VARIANT_AND);
}

/// Checks for `.is_ok_and(|x| x == y)` or `.is_ok_and(|x| y == x)`
pub(super) fn check_is_ok_and<'a>(cx: &LateContext<'a>, expr: &Expr<'a>, recv: &Expr<'_>, map: &'a Expr<'a>) {
    let def_bool = false; // `is_ok_and` is equiv to `map_or(false, `
    check_inner(cx, expr, recv, "is_ok_and", map, def_bool, NEEDLESS_IS_VARIANT_AND);
}

/// Checks whether:
/// - the receiver is either an `Option` or `Result`
/// - the `(def_bool, closure)`-pair looks like one of:
///   - `(false, |x| x == y)` or `(false, |x| y == x)` -- can be replaced with `== Some/Ok(y)`
///   - `(true,  |x| x != y)` or `(true,  |x| y != x)` -- can be replaced with `!= Some/Ok(y)`
fn check_inner<'a>(
    cx: &LateContext<'a>,
    expr: &Expr<'a>,
    recv: &Expr<'_>,
    method_name: &'static str,
    closure: &'a Expr<'a>,
    def_bool: bool,
    lint: &'static Lint,
) -> bool {
    let typeck = cx.typeck_results();
    let recv_ty = typeck.expr_ty_adjusted(recv);
    let wrap = match recv_ty.opt_diag_name(cx) {
        Some(sym::Option) => "Some",
        Some(sym::Result) => "Ok",
        Some(_) | None => return false,
    };
    if typeck.expr_adjustments(recv).is_empty()
        && let ExprKind::Closure(closure) = closure.kind
        && let closure_body = cx.tcx.hir_body(closure.body)
        && let closure_body_value = closure_body.value.peel_blocks()
        && let ExprKind::Binary(op, l, r) = closure_body_value.kind
        && let [param] = closure_body.params
        && let PatKind::Binding(_, hir_id, _, _) = param.pat.kind
        && ((BinOpKind::Eq == op.node && !def_bool) || (BinOpKind::Ne == op.node && def_bool))
        && let non_binding_location = if l.res_local_id() == Some(hir_id) { r } else { l }
        && switch_to_eager_eval(cx, non_binding_location)
        // if it's both then that's a strange edge case and
        // we can just ignore it, since by default clippy will error on this
        && (l.res_local_id() == Some(hir_id)) != (r.res_local_id() == Some(hir_id))
        && !is_local_used(cx, non_binding_location, hir_id)
        && let l_ty = typeck.expr_ty(l)
        && l_ty == typeck.expr_ty(r)
        && let Some(partial_eq) = cx.tcx.lang_items().eq_trait()
        && implements_trait(cx, recv_ty, partial_eq, &[recv_ty.into()])
        && is_copy(cx, l_ty)
        && !is_from_proc_macro(cx, expr)
    {
        let mut fired = false;
        span_lint_and_then(
            cx,
            lint,
            expr.span,
            format!("this `{method_name}` can be simplified"),
            |diag| {
                // we may need to add parens around the suggestion
                // in case the parent expression has additional method calls,
                // since for example `Some(5).map_or(false, |x| x == 5).then(|| 1)`
                // being converted to `Some(5) == Some(5).then(|| 1)` isn't
                // the same thing

                let mut app = Applicability::MachineApplicable;
                let inner_non_binding = Sugg::NonParen(Cow::Owned(format!(
                    "{wrap}({})",
                    Sugg::hir_with_applicability(cx, non_binding_location, "", &mut app)
                )));

                let binop = make_binop(
                    op.node,
                    &Sugg::hir_with_applicability(cx, recv, "..", &mut app),
                    &inner_non_binding,
                );

                let sugg = if let Some(parent_expr) = get_parent_expr(cx, expr) {
                    if parent_expr.span.eq_ctxt(expr.span) {
                        match parent_expr.kind {
                            ExprKind::Binary(..) | ExprKind::Unary(..) | ExprKind::Cast(..) => binop.maybe_paren(),
                            ExprKind::MethodCall(_, receiver, _, _) if receiver.hir_id == expr.hir_id => {
                                binop.maybe_paren()
                            },
                            _ => binop,
                        }
                    } else {
                        // if our parent expr is created by a macro, then it should be the one taking care of
                        // parenthesising us if necessary
                        binop
                    }
                } else {
                    binop
                };
                diag.span_suggestion_verbose(expr.span, "use a standard comparison instead", sugg.to_string(), app);

                fired = true;
            },
        );
        return fired;
    }
    false
}
