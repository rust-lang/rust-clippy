use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef, MaybeResPath, MaybeTypeckRes};
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Body, Closure, Expr, ExprKind};
use rustc_lint::{LateContext, Lint};
use rustc_middle::ty::Ty;
use rustc_span::Span;

use super::{LINES_FILTER_MAP_OK, SPLIT_FILTER_MAP_OK};

#[derive(Clone, Copy)]
enum Variant {
    Lines,
    Split,
}

impl Variant {
    fn lint(self) -> &'static Lint {
        match self {
            Variant::Lines => LINES_FILTER_MAP_OK,
            Variant::Split => SPLIT_FILTER_MAP_OK,
        }
    }

    fn type_name(self) -> &'static str {
        match self {
            Variant::Lines => "std::io::Lines",
            Variant::Split => "std::io::Split",
        }
    }
}

fn get_variant(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<Variant> {
    match ty.opt_diag_name(cx) {
        Some(sym::IoLines) => Some(Variant::Lines),
        Some(sym::IoSplit) => Some(Variant::Split),
        _ => None,
    }
}

pub(super) fn check_flatten(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, call_span: Span, msrv: Msrv) {
    if cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
        && let Some(variant) = get_variant(cx, cx.typeck_results().expr_ty_adjusted(recv))
        && msrv.meets(cx, msrvs::MAP_WHILE)
    {
        emit(cx, recv, "flatten", call_span, variant);
    }
}

pub(super) fn check_filter_or_flat_map(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    recv: &Expr<'_>,
    method_name: &'static str,
    method_arg: &Expr<'_>,
    call_span: Span,
    msrv: Msrv,
) {
    if cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
        && let Some(variant) = get_variant(cx, cx.typeck_results().expr_ty_adjusted(recv))
        && match method_arg.kind {
            // Detect `Result::ok`
            ExprKind::Path(ref qpath) => cx
                .qpath_res(qpath, method_arg.hir_id)
                .is_diag_item(cx, sym::result_ok_method),
            // Detect `|x| x.ok()`
            ExprKind::Closure(&Closure { body, .. }) => {
                if let Body {
                    params: [param], value, ..
                } = cx.tcx.hir_body(body)
                    && let ExprKind::MethodCall(method, receiver, [], _) = value.kind
                {
                    method.ident.name == sym::ok
                        && receiver.res_local_id() == Some(param.pat.hir_id)
                        && cx.ty_based_def(*value).is_diag_item(cx, sym::result_ok_method)
                } else {
                    false
                }
            },
            _ => false,
        }
        && msrv.meets(cx, msrvs::MAP_WHILE)
    {
        emit(cx, recv, method_name, call_span, variant);
    }
}

fn emit(cx: &LateContext<'_>, recv: &Expr<'_>, method_name: &'static str, call_span: Span, variant: Variant) {
    span_lint_and_then(
        cx,
        variant.lint(),
        call_span,
        format!("`{method_name}()` will run forever if the iterator repeatedly produces an `Err`"),
        |diag| {
            diag.span_note(
                recv.span,
                format!(
                    "this expression returning a `{0}` may produce \
                        an infinite number of `Err` in case of a read error",
                    variant.type_name(),
                ),
            );
            diag.span_suggestion(
                call_span,
                "replace with",
                "map_while(Result::ok)",
                Applicability::MaybeIncorrect,
            );
        },
    );
}
