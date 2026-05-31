use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LintContext};

use crate::operators::BITWISE_NOT_ZERO;

/// `arg` is argument to the `!` operator, `expr` is the entire expression
pub(super) fn check(cx: &LateContext<'_>, expr: Expr<'_>, arg: Expr<'_>) {
    if !clippy_utils::consts::is_zero_integer_const(cx, &arg, arg.span.ctxt()) {
        return;
    }

    // If argument to `!` is from a macro expansion, bail
    if arg.span.in_external_macro(cx.sess().source_map()) {
        return;
    }

    let arg_ty = cx.typeck_results().expr_ty(&arg);

    let integer = match arg_ty.kind() {
        rustc_middle::ty::Uint(uint_ty) => uint_ty.name_str(),
        _ => return,
    };

    span_lint_and_sugg(
        cx,
        BITWISE_NOT_ZERO,
        expr.span,
        "usage of the bitwise not `!` on zero",
        "this is clearer written as the maximum value",
        format!("{integer}::MAX"),
        Applicability::MaybeIncorrect,
    );
}
