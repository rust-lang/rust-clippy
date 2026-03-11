use super::utils::{
    BindingKey, BindingSource, expr_borrows, expr_is_field_access, mapping_of_mirrored_pats, mirrored_exprs,
};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::std_or_core;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{implements_trait, is_copy, peel_n_ty_refs};
use rustc_errors::Applicability;
use rustc_hir::{Closure, Expr, ExprKind, Param, PatKind, Path, PathSegment, QPath};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::UNNECESSARY_SORT_BY;

enum LintTrigger {
    Sort,
    SortByKey(SortByKeyDetection),
}

struct SortByKeyDetection {
    closure_arg: Span,
    closure_body: String,
    reverse: bool,
    applicability: Applicability,
}

fn detect_lint(cx: &LateContext<'_>, expr: &Expr<'_>, arg: &Expr<'_>) -> Option<LintTrigger> {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_assoc(method_id)
        && cx.tcx.type_of(impl_id).instantiate_identity().is_slice()
        && let ExprKind::Closure(&Closure { body, .. }) = arg.kind
        && let closure_body = cx.tcx.hir_body(body)
        && let &[Param { pat: l_pat, .. }, Param { pat: r_pat, .. }] = closure_body.params
        && let Some(binding_map) = mapping_of_mirrored_pats(l_pat, r_pat)
        && let ExprKind::MethodCall(method_path, left_expr, [right_expr], _) = closure_body.value.kind
        && method_path.ident.name == sym::cmp
        && let Some(ord_trait) = cx.tcx.get_diagnostic_item(sym::Ord)
        && cx.ty_based_def(closure_body.value).opt_parent(cx).opt_def_id() == Some(ord_trait)
    {
        let (closure_body, closure_arg, reverse) =
            if mirrored_exprs(left_expr, right_expr, &binding_map, BindingSource::Left) {
                (left_expr, l_pat.span, false)
            } else if mirrored_exprs(left_expr, right_expr, &binding_map, BindingSource::Right) {
                (left_expr, r_pat.span, true)
            } else {
                return None;
            };

        let mut applicability = if reverse {
            Applicability::MaybeIncorrect
        } else {
            Applicability::MachineApplicable
        };

        if let ExprKind::Path(QPath::Resolved(
            _,
            Path {
                segments: [PathSegment { ident: left_name, .. }],
                ..
            },
        )) = left_expr.kind
        {
            if let PatKind::Binding(_, _, left_ident, _) = l_pat.kind
                && *left_name == left_ident
                && implements_trait(cx, cx.typeck_results().expr_ty(left_expr), ord_trait, &[])
            {
                return Some(LintTrigger::Sort);
            }

            let mut left_expr_ty = cx.typeck_results().expr_ty(left_expr);
            let left_ident_n_refs = binding_map
                .get(&BindingKey {
                    ident: *left_name,
                    source: BindingSource::Left,
                })
                .map_or(0, |value| value.n_refs);
            // Peel off the outer-most ref which is introduced by the closure, if it is not already peeled
            // by the pattern
            if left_ident_n_refs == 0 {
                (left_expr_ty, _) = peel_n_ty_refs(left_expr_ty, 1);
            }
            if !reverse && is_copy(cx, left_expr_ty) {
                let mut closure_body =
                    snippet_with_applicability(cx, closure_body.span, "_", &mut applicability).to_string();
                if left_ident_n_refs == 0 {
                    closure_body = format!("*{closure_body}");
                }
                return Some(LintTrigger::SortByKey(SortByKeyDetection {
                    closure_arg,
                    closure_body,
                    reverse,
                    applicability,
                }));
            }
        }

        let left_expr_ty = cx.typeck_results().expr_ty(left_expr);
        if !expr_borrows(left_expr_ty)
            // Don't lint if the closure is accessing non-Copy fields
            && (!expr_is_field_access(left_expr) || is_copy(cx, left_expr_ty))
        {
            let closure_body = Sugg::hir_with_applicability(cx, closure_body, "_", &mut applicability).to_string();
            return Some(LintTrigger::SortByKey(SortByKeyDetection {
                closure_arg,
                closure_body,
                reverse,
                applicability,
            }));
        }
    }

    None
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    call_span: Span,
    arg: &'tcx Expr<'_>,
    is_unstable: bool,
) {
    match detect_lint(cx, expr, arg) {
        Some(LintTrigger::SortByKey(trigger)) => {
            let method = if is_unstable {
                "sort_unstable_by_key"
            } else {
                "sort_by_key"
            };
            let Some(std_or_core) = std_or_core(cx) else {
                // To make it this far the crate has to reference diagnostic items defined in core. Either this is
                // the `core` crate, there's an `extern crate core` somewhere, or another crate is defining the
                // diagnostic items. It's fine to not lint in all those cases even if we might be able to.
                return;
            };
            span_lint_and_then(
                cx,
                UNNECESSARY_SORT_BY,
                expr.span,
                format!("consider using `{method}`"),
                |diag| {
                    let mut app = trigger.applicability;
                    let closure_body = if trigger.reverse {
                        format!("{std_or_core}::cmp::Reverse({})", trigger.closure_body)
                    } else {
                        trigger.closure_body
                    };
                    let closure_arg = snippet_with_applicability(cx, trigger.closure_arg, "_", &mut app);
                    diag.span_suggestion_verbose(
                        call_span,
                        "try",
                        format!("{method}(|{closure_arg}| {closure_body})"),
                        app,
                    );
                },
            );
        },
        Some(LintTrigger::Sort) => {
            let method = if is_unstable { "sort_unstable" } else { "sort" };
            span_lint_and_then(
                cx,
                UNNECESSARY_SORT_BY,
                expr.span,
                format!("consider using `{method}`"),
                |diag| {
                    diag.span_suggestion_verbose(
                        call_span,
                        "try",
                        format!("{method}()"),
                        Applicability::MachineApplicable,
                    );
                },
            );
        },
        None => {},
    }
}
