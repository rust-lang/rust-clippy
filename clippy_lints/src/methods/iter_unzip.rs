use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::ITER_UNZIP;

pub(super) fn check(cx: &LateContext<'_>, msrv: Msrv, expr: &hir::Expr<'_>, span: Span) {
    if cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
        && msrv.meets(cx, msrvs::TUPLE_FROM_ITERATOR)
    {
        let mut app = Applicability::MachineApplicable;

        let (suggestion, span) = if let hir::ExprKind::MethodCall(
            hir::PathSegment {
                args:
                    Some(hir::GenericArgs {
                        args: [_arg_a, _arg_b, arg_from_a, arg_from_b],
                        span_ext,
                        ..
                    }),
                ..
            },
            ..,
        ) = expr.kind
        {
            let [snip_a, snip_b] = [arg_from_a, arg_from_b].map(|arg| {
                if let hir::GenericArg::Type(ty) = arg {
                    snippet_with_applicability(cx, ty.span, "_", &mut app)
                } else {
                    "_".into()
                }
            });

            (format!("collect::<({snip_a}, {snip_b})>"), span.to(*span_ext))
        } else {
            ("collect".to_string(), span)
        };

        span_lint_and_sugg(cx, ITER_UNZIP, span, "use of `Iterator::unzip`", "try", suggestion, app);
    }
}
