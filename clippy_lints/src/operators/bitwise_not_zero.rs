use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::ty::Ty;

use crate::operators::BITWISE_NOT_ZERO;

/// `arg` is argument to the `!` operator, `expr` is the entire expression,
/// and `arg_ty` is the type of `arg`.
pub(super) fn check(cx: &LateContext<'_>, arg_ty: Ty<'_>, expr: Expr<'_>, arg: Expr<'_>) {
    if !clippy_utils::consts::is_zero_integer_const(cx, &arg, arg.span.ctxt()) {
        return;
    }

    // If argument to `!` is from a macro expansion, bail
    // if arg.span.in_external_macro(cx.sess().source_map()) {
    //     return;
    // }
    if arg.span.in_external_macro(cx.sess().source_map()) {
        return;
    }

    let integer = match arg_ty.kind() {
        rustc_middle::ty::Int(int_ty) => match int_ty {
            rustc_ast::IntTy::Isize => "isize",
            rustc_ast::IntTy::I8 => "i8",
            rustc_ast::IntTy::I16 => "i16",
            rustc_ast::IntTy::I32 => "i32",
            rustc_ast::IntTy::I64 => "i64",
            rustc_ast::IntTy::I128 => "i128",
        },
        rustc_middle::ty::Uint(uint_ty) => match uint_ty {
            rustc_ast::UintTy::Usize => "usize",
            rustc_ast::UintTy::U8 => "u8",
            rustc_ast::UintTy::U16 => "u16",
            rustc_ast::UintTy::U32 => "u32",
            rustc_ast::UintTy::U64 => "u64",
            rustc_ast::UintTy::U128 => "u128",
        },
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
