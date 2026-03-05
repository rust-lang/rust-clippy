use super::SUSPICIOUS_SLICE_COPIES;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::{IntoSpan, SpanRangeExt, snippet};
use clippy_utils::{get_parent_expr, sym};
use rustc_ast::attr::version::RustcVersion;
use rustc_ast::{BorrowKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_middle::ty::GenericArgKind;

const FEATURE_SLICE_AS_MUT_ARRAY: RustcVersion = RustcVersion {
    major: 1,
    minor: 93,
    patch: 0,
};

enum TryIntoReceiver {
    Slice,
    SliceMutRef,
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, msrv: Msrv) {
    if let ExprKind::MethodCall(_, recv_expr, _, try_into_span) = expr.kind

        // try_into is called with slice or slice mut ref
        && let Some(receiver) = receiver(cx, recv_expr)

        // try_into returns Result<[T; N], TryFromSliceError>
        && let ty::Adt(r_def, args) = cx.typeck_results().expr_ty(expr).kind()
        && cx.tcx.is_diagnostic_item(sym::Result, r_def.did())
        && let Some(arg) = args.first()
        && let GenericArgKind::Type(ref arg_ty) = arg.kind()
        && let ty::Array(..) = arg_ty.kind()

        // calls unwrap() or expect()
        && let Some(parent_call) = get_parent_expr(cx, expr)
        && let ExprKind::MethodCall(path, .., unwrap) = parent_call.kind
        && let sym::unwrap | sym::expect = path.ident.name

        // use mut ref
        && let Some(mut_ref_expr) = get_parent_expr(cx, parent_call)
        && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, inner_expr) = mut_ref_expr.kind
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_SLICE_COPIES,
            mut_ref_expr.span.with_hi(unwrap.hi()),
            "using a mutable reference to temporary array",
            |diag| {
                let recv_snippet = snippet(cx, recv_expr.span, "..");

                let (span, sugg) = if msrv.meets(cx, FEATURE_SLICE_AS_MUT_ARRAY) {
                    (
                        mut_ref_expr.span.with_hi(try_into_span.hi()),
                        format!("{recv_snippet}.as_mut_array()"),
                    )
                } else {
                    (
                        mut_ref_expr.span.with_hi(recv_expr.span.hi()),
                        match receiver {
                            TryIntoReceiver::Slice => format!(
                                "({mut_ref_snippet} {recv_snippet})",
                                mut_ref_snippet = snippet(
                                    cx,
                                    mut_ref_expr
                                        .span
                                        .until(inner_expr.span.with_leading_whitespace(cx).into_span()),
                                    ".."
                                )
                            ),
                            TryIntoReceiver::SliceMutRef => recv_snippet.to_string(),
                        },
                    )
                };

                diag.span_suggestion(span, "try", sugg, Applicability::MachineApplicable);
            },
        );
    }
}

fn receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<TryIntoReceiver> {
    match cx.typeck_results().expr_ty(expr).kind() {
        ty::Slice(_) => Some(TryIntoReceiver::Slice),
        ty::Ref(_, ref_ty, Mutability::Mut) if let ty::Slice(_) = ref_ty.kind() => Some(TryIntoReceiver::SliceMutRef),
        _ => None,
    }
}
