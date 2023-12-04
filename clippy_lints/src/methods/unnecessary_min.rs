use super::UNNECESSARY_MIN;
use clippy_utils::diagnostics::span_lint_and_sugg;

use hir::{Expr, ExprKind};

use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;

pub fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, _: &'tcx Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(expr);
    if !matches!(ty.kind(), ty::Uint(_)) {
        return;
    }
    match expr.kind {
        hir::ExprKind::MethodCall(_, left1, right1, _) => {
            let left = left1.kind;
            let right = right1[0].kind;
            if let ExprKind::Lit(test) = left {
                if let LitKind::Int(0, _) = test.node {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_MIN,
                        expr.span,
                        "this operation has no effect",
                        "try: ",
                        "0".to_string(),
                        Applicability::MachineApplicable,
                    );
                }
            }

            if let ExprKind::Lit(test) = right {
                if let LitKind::Int(0, _) = test.node {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_MIN,
                        expr.span,
                        "this operation has no effect",
                        "try: ",
                        "0".to_string(),
                        Applicability::MachineApplicable,
                    );
                }
            }
        },
        _ => {},
    };
}
