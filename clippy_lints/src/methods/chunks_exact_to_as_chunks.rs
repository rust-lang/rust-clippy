use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::msrvs::{self, Msrv};
use rustc_hir::Expr;
use rustc_lint::LateContext;

use super::CHUNKS_EXACT_TO_AS_CHUNKS;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    recv: &Expr<'_>,
    arg: &Expr<'_>,
    method_name: &str,
    msrv: &Msrv,
) {
    // Check for Rust version
    if !msrv.meets(cx, msrvs::AS_CHUNKS) {
        return;
    }

    // Check receiver is slice or array type
    let recv_ty = cx.typeck_results().expr_ty(recv).peel_refs();
    if !recv_ty.is_slice() && !recv_ty.is_array() {
        return;
    }

    // Check if argument is a constant
    let constant_eval = ConstEvalCtxt::new(cx);
    if let Some(Constant::Int(chunk_size)) = constant_eval.eval(arg) {
        // Emit the lint
        let suggestion = if method_name == "chunks_exact_mut" {
            "as_chunks_mut"
        } else {
            "as_chunks"
        };

        span_lint_and_help(
            cx,
            CHUNKS_EXACT_TO_AS_CHUNKS,
            expr.span,
            format!("using `{method_name}` with a constant chunk size"),
            None,
            format!("consider using `{suggestion}::<{chunk_size}>()` for better ergonomics"),
        );
    }
}
