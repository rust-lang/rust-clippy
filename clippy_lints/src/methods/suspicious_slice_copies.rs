use super::SUSPICIOUS_SLICE_COPIES;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::{IntoSpan, SpanRangeExt, snippet_with_context};
use clippy_utils::{msrvs, sym};
use rustc_ast::{BorrowKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_middle::ty::{GenericArgKind, Ty};
use rustc_span::Span;

enum TryIntoReceiver {
    Slice,
    SliceMutRef,
}

impl TryIntoReceiver {
    fn from_ty(ty: Ty<'_>) -> Option<Self> {
        match ty.kind() {
            ty::Slice(_) => Some(TryIntoReceiver::Slice),
            ty::Ref(_, ref_ty, Mutability::Mut) if let ty::Slice(_) = ref_ty.kind() => {
                Some(TryIntoReceiver::SliceMutRef)
            },
            _ => None,
        }
    }
}

pub(super) fn check(
    cx: &LateContext<'_>,
    try_into_expr: &Expr<'_>,
    recv_expr: &Expr<'_>,
    try_into_span: Span,
    msrv: Msrv,
) {
    if let Some(receiver) = TryIntoReceiver::from_ty(cx.typeck_results().expr_ty(recv_expr))

        // try_into was instantiated with [T; N]
        && let [_, dst_ty] = &**cx.typeck_results().node_args(try_into_expr.hir_id)
        && let GenericArgKind::Type(arg_ty) = dst_ty.kind()
        && let ty::Array(..) = arg_ty.kind()

        && let mut parent_iter = cx.tcx.hir_parent_iter(try_into_expr.hir_id)

        // calls unwrap() or expect()
        && let Some((_, Node::Expr(unwrap_expr))) = parent_iter.next()
        && let ExprKind::MethodCall(path, .., unwrap_span) = unwrap_expr.kind
        && let sym::unwrap | sym::expect = path.ident.name

        // use mut ref
        && let Some((_, Node::Expr(mut_ref_expr))) = parent_iter.next()
        && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, inner_expr) = mut_ref_expr.kind
    {
        let span = mut_ref_expr.span.with_hi(inner_expr.span.hi());

        span_lint_and_then(
            cx,
            SUSPICIOUS_SLICE_COPIES,
            span,
            "using a mutable reference to temporary array",
            |diag| {
                let mut applicability = Applicability::MaybeIncorrect;

                let (recv_snippet, _) =
                    snippet_with_context(cx, recv_expr.span, try_into_span.ctxt(), "..", &mut applicability);
                let (unwrap_snippet, _) =
                    snippet_with_context(cx, unwrap_span, try_into_span.ctxt(), "..", &mut applicability);
                let (mut_ref_snippet, _) = snippet_with_context(
                    cx,
                    mut_ref_expr
                        .span
                        .until(inner_expr.span.with_leading_whitespace(cx).into_span()),
                    try_into_span.ctxt(),
                    "..",
                    &mut applicability,
                );

                let borrow_sugg = if msrv.meets(cx, msrvs::SLICE_AS_MUT_ARRAY) {
                    format!("{recv_snippet}.as_mut_array().{unwrap_snippet}")
                } else {
                    match receiver {
                        TryIntoReceiver::Slice => {
                            format!("({mut_ref_snippet} {recv_snippet}).try_into().{unwrap_snippet}")
                        },
                        TryIntoReceiver::SliceMutRef => format!("{recv_snippet}.try_into().{unwrap_snippet}"),
                    }
                };

                let copy_sugg = match receiver {
                    TryIntoReceiver::Slice => {
                        format!("{mut_ref_snippet} <[_; _]>::try_from(&{recv_snippet}).{unwrap_snippet}")
                    },
                    TryIntoReceiver::SliceMutRef => {
                        format!("{mut_ref_snippet} <[_; _]>::try_from({recv_snippet}).{unwrap_snippet}")
                    },
                };

                diag.span_suggestion(span, "to borrow the slice as an array, try", borrow_sugg, applicability);
                diag.span_suggestion(
                    span,
                    "to create a new array from the slice, try",
                    copy_sugg,
                    applicability,
                );
            },
        );
    }
}
