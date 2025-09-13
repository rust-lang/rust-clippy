use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Symbol;
use std::fmt;

use super::PTR_OFFSET_WITH_CAST;

pub(super) fn check(cx: &LateContext<'_>, method: Symbol, expr: &Expr<'_>, recv: &Expr<'_>, arg: &Expr<'_>) {
    let ty::RawPtr(_, _) = cx.typeck_results().expr_ty(recv).kind() else {
        return;
    };

    let method = match method {
        sym::offset => Method::Offset,
        sym::wrapping_offset => Method::WrappingOffset,
        _ => return,
    };

    // Check if the argument to the method call is a cast from usize
    let cast_lhs_expr = match arg.kind {
        ExprKind::Cast(lhs, _) if is_expr_ty_usize(cx, lhs) => lhs,
        _ => return,
    };

    let msg = format!("use of `{method}` with a `usize` casted to an `isize`");
    span_lint_and_then(cx, PTR_OFFSET_WITH_CAST, expr.span, msg, |diag| {
        if let Some(sugg) = build_suggestion(cx, method, recv, cast_lhs_expr) {
            diag.span_suggestion(expr.span, "try", sugg, Applicability::MachineApplicable);
        }
    });
}

// Is the type of the expression a usize?
fn is_expr_ty_usize(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results().expr_ty(expr) == cx.tcx.types.usize
}

fn build_suggestion(
    cx: &LateContext<'_>,
    method: Method,
    receiver_expr: &Expr<'_>,
    cast_lhs_expr: &Expr<'_>,
) -> Option<String> {
    let receiver = receiver_expr.span.get_source_text(cx)?;
    let cast_lhs = cast_lhs_expr.span.get_source_text(cx)?;
    Some(format!("{receiver}.{}({cast_lhs})", method.suggestion()))
}

#[derive(Copy, Clone)]
enum Method {
    Offset,
    WrappingOffset,
}

impl Method {
    #[must_use]
    fn suggestion(self) -> &'static str {
        match self {
            Self::Offset => "add",
            Self::WrappingOffset => "wrapping_add",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset => write!(f, "offset"),
            Self::WrappingOffset => write!(f, "wrapping_offset"),
        }
    }
}
