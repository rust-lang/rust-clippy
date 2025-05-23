use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eager_or_lazy::switch_to_lazy_eval;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_context;
use clippy_utils::ty::{expr_type_is_certain, implements_trait, is_type_diagnostic_item};
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{
    contains_return, is_default_equivalent, is_default_equivalent_call, last_path_segment, peel_blocks, sym,
};
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;
use rustc_span::symbol::{self, Symbol};
use {rustc_ast as ast, rustc_hir as hir};

use super::{OR_FUN_CALL, UNWRAP_OR_DEFAULT};

/// Checks for the `OR_FUN_CALL` lint.
#[expect(clippy::too_many_lines)]
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &hir::Expr<'_>,
    method_span: Span,
    name: Symbol,
    receiver: &'tcx hir::Expr<'_>,
    args: &'tcx [hir::Expr<'_>],
    msrv: Msrv,
) {
    /// Checks for `unwrap_or(T::new())`, `unwrap_or(T::default())`,
    /// `or_insert(T::new())` or `or_insert(T::default())`.
    /// Similarly checks for `unwrap_or_else(T::new)`, `unwrap_or_else(T::default)`,
    /// `or_insert_with(T::new)` or `or_insert_with(T::default)`.
    fn check_unwrap_or_default(
        cx: &LateContext<'_>,
        name: Symbol,
        receiver: &hir::Expr<'_>,
        fun: &hir::Expr<'_>,
        call_expr: Option<&hir::Expr<'_>>,
        span: Span,
        method_span: Span,
        msrv: Msrv,
    ) -> bool {
        if !expr_type_is_certain(cx, receiver) {
            return false;
        }

        let is_new = |fun: &hir::Expr<'_>| {
            if let hir::ExprKind::Path(ref qpath) = fun.kind {
                let path = last_path_segment(qpath).ident.name;
                matches!(path, sym::new)
            } else {
                false
            }
        };

        let output_type_implements_default = |fun| {
            let fun_ty = cx.typeck_results().expr_ty(fun);
            if let ty::FnDef(def_id, args) = fun_ty.kind() {
                let output_ty = cx.tcx.fn_sig(def_id).instantiate(cx.tcx, args).skip_binder().output();
                cx.tcx
                    .get_diagnostic_item(sym::Default)
                    .is_some_and(|default_trait_id| implements_trait(cx, output_ty, default_trait_id, &[]))
            } else {
                false
            }
        };

        let sugg = match (name, call_expr.is_some()) {
            (sym::unwrap_or, true) | (sym::unwrap_or_else, false) => sym::unwrap_or_default,
            (sym::or_insert, true) | (sym::or_insert_with, false) => sym::or_default,
            _ => return false,
        };

        let receiver_ty = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
        let Some(suggested_method_def_id) = receiver_ty.ty_adt_def().and_then(|adt_def| {
            cx.tcx
                .inherent_impls(adt_def.did())
                .iter()
                .flat_map(|impl_id| cx.tcx.associated_items(impl_id).filter_by_name_unhygienic(sugg))
                .find_map(|assoc| {
                    if assoc.is_method() && cx.tcx.fn_sig(assoc.def_id).skip_binder().inputs().skip_binder().len() == 1
                    {
                        Some(assoc.def_id)
                    } else {
                        None
                    }
                })
        }) else {
            return false;
        };
        let in_sugg_method_implementation = {
            matches!(
                suggested_method_def_id.as_local(),
                Some(local_def_id) if local_def_id == cx.tcx.hir_get_parent_item(receiver.hir_id).def_id
            )
        };
        if in_sugg_method_implementation {
            return false;
        }

        // Check if MSRV meets the requirement for unwrap_or_default (stabilized in 1.16)
        if !msrv.meets(cx, msrvs::UNWRAP_OR_DEFAULT) {
            return false;
        }

        // needs to target Default::default in particular or be *::new and have a Default impl
        // available
        if (is_new(fun) && output_type_implements_default(fun))
            || match call_expr {
                Some(call_expr) => is_default_equivalent(cx, call_expr),
                None => is_default_equivalent_call(cx, fun, None) || closure_body_returns_empty_to_string(cx, fun),
            }
        {
            span_lint_and_sugg(
                cx,
                UNWRAP_OR_DEFAULT,
                method_span.with_hi(span.hi()),
                format!("use of `{name}` to construct default value"),
                "try",
                format!("{sugg}()"),
                Applicability::MachineApplicable,
            );

            true
        } else {
            false
        }
    }

    /// Checks for `*or(foo())`.
    #[expect(clippy::too_many_arguments)]
    fn check_or_fn_call(
        cx: &LateContext<'_>,
        name: Symbol,
        method_span: Span,
        receiver: &hir::Expr<'_>,
        arg: &hir::Expr<'_>,
        fun_span: Option<Span>,
        span: Span,
        msrv: Msrv,
    ) -> bool {
        if !expr_type_is_certain(cx, receiver) {
            return false;
        }

        let is_new = |fun: &hir::Expr<'_>| {
            if let hir::ExprKind::Path(ref qpath) = fun.kind {
                let path = last_path_segment(qpath).ident.name;
                matches!(path, sym::new)
            } else {
                false
            }
        };

        let output_type_implements_default = |fun| {
            let fun_ty = cx.typeck_results().expr_ty(fun);
            if let ty::FnDef(def_id, args) = fun_ty.kind() {
                let output_ty = cx.tcx.fn_sig(def_id).instantiate(cx.tcx, args).skip_binder().output();
                cx.tcx
                    .get_diagnostic_item(sym::Default)
                    .is_some_and(|default_trait_id| implements_trait(cx, output_ty, default_trait_id, &[]))
            } else {
                false
            }
        };

        let sugg = match (name, fun_span.is_some()) {
            (sym::unwrap_or, true) | (sym::unwrap_or_else, false) => sym::unwrap_or_default,
            (sym::or_insert, true) | (sym::or_insert_with, false) => sym::or_default,
            _ => return false,
        };

        let receiver_ty = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
        let Some(suggested_method_def_id) = receiver_ty.ty_adt_def().and_then(|adt_def| {
            cx.tcx
                .inherent_impls(adt_def.did())
                .iter()
                .flat_map(|impl_id| cx.tcx.associated_items(impl_id).filter_by_name_unhygienic(sugg))
                .find_map(|assoc| {
                    if assoc.is_method() && cx.tcx.fn_sig(assoc.def_id).skip_binder().inputs().skip_binder().len() == 1
                    {
                        Some(assoc.def_id)
                    } else {
                        None
                    }
                })
        }) else {
            return false;
        };
        let in_sugg_method_implementation = {
            matches!(
                suggested_method_def_id.as_local(),
                Some(local_def_id) if local_def_id == cx.tcx.hir_get_parent_item(receiver.hir_id).def_id
            )
        };
        if in_sugg_method_implementation {
            return false;
        }

        // Check if MSRV meets the requirement for unwrap_or_default (stabilized in 1.16)
        if !msrv.meets(cx, msrvs::UNWRAP_OR_DEFAULT) {
            return false;
        }

        // needs to target Default::default in particular or be *::new and have a Default impl
        // available
        if (is_new(arg) && output_type_implements_default(arg))
            || match fun_span {
                Some(_) => is_default_equivalent(cx, arg),
                None => is_default_equivalent_call(cx, arg, None) || closure_body_returns_empty_to_string(cx, arg),
            }
        {
            span_lint_and_sugg(
                cx,
                OR_FUN_CALL,
                method_span.with_hi(span.hi()),
                format!("use of `{name}` to construct default value"),
                "try",
                format!("{sugg}()"),
                Applicability::MachineApplicable,
            );

            true
        } else {
            false
        }
    }

    if let [arg] = args {
        let inner_arg = peel_blocks(arg);
        for_each_expr(cx, inner_arg, |ex| {
            // `or_fun_call` lint needs to take nested expr into account,
            // but `unwrap_or_default` lint doesn't, we don't want something like:
            // `opt.unwrap_or(Foo { inner: String::default(), other: 1 })` to get replaced by
            // `opt.unwrap_or_default()`.
            let is_nested_expr = ex.hir_id != inner_arg.hir_id;

            let is_triggered = match ex.kind {
                hir::ExprKind::Call(fun, fun_args) => {
                    let inner_fun_has_args = !fun_args.is_empty();
                    let fun_span = if inner_fun_has_args || is_nested_expr {
                        None
                    } else {
                        Some(fun.span)
                    };
                    (!inner_fun_has_args
                        && !is_nested_expr
                        && check_unwrap_or_default(cx, name, receiver, fun, Some(ex), expr.span, method_span, msrv))
                        || check_or_fn_call(cx, name, method_span, receiver, arg, None, expr.span, msrv)
                },
                hir::ExprKind::Path(..) | hir::ExprKind::Closure(..) if !is_nested_expr => {
                    check_unwrap_or_default(cx, name, receiver, ex, None, expr.span, method_span, msrv)
                },
                hir::ExprKind::Index(..) | hir::ExprKind::MethodCall(..) => {
                    check_or_fn_call(cx, name, method_span, receiver, arg, None, expr.span, msrv)
                },
                _ => false,
            };

            if is_triggered {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
    }
}

fn closure_body_returns_empty_to_string(cx: &LateContext<'_>, e: &hir::Expr<'_>) -> bool {
    if let hir::ExprKind::Closure(&hir::Closure { body, .. }) = e.kind {
        let body = cx.tcx.hir_body(body);

        if body.params.is_empty()
            && let hir::Expr { kind, .. } = &body.value
            && let hir::ExprKind::MethodCall(hir::PathSegment { ident, .. }, self_arg, [], _) = kind
            && ident.name == sym::to_string
            && let hir::Expr { kind, .. } = self_arg
            && let hir::ExprKind::Lit(lit) = kind
            && let ast::LitKind::Str(symbol::kw::Empty, _) = lit.node
        {
            return true;
        }
    }

    false
}
