use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, Node, PatKind};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

use super::CHUNKS_EXACT_WITH_CONST_SIZE;

pub(super) fn check(
    cx: &LateContext<'_>,
    recv: &Expr<'_>,
    arg: &Expr<'_>,
    expr: &Expr<'_>,
    call_span: Span,
    method_name: Symbol,
    msrv: Msrv,
) {
    // Check if receiver is slice-like
    if !cx.typeck_results().expr_ty_adjusted(recv).peel_refs().is_slice() {
        return;
    }

    let constant_eval = ConstEvalCtxt::new(cx);
    if let Some(Constant::Int(_)) = constant_eval.eval(arg) {
        // Check for Rust version
        if !msrv.meets(cx, msrvs::AS_CHUNKS) {
            return;
        }

        let suggestion_method = if method_name == sym::chunks_exact_mut {
            "as_chunks_mut"
        } else {
            "as_chunks"
        };

        let mut applicability = Applicability::MachineApplicable;
        let arg_str = snippet_with_applicability(cx, arg.span, "_", &mut applicability);

        let as_chunks = format_args!("{suggestion_method}::<{arg_str}>()");

        span_lint_and_then(
            cx,
            CHUNKS_EXACT_WITH_CONST_SIZE,
            call_span,
            format!("using `{method_name}` with a constant chunk size"),
            |diag| {
                if let Node::LetStmt(let_stmt) = cx.tcx.parent_hir_node(expr.hir_id) {
                    // The `ChunksExact(Mut)` struct is stored for later -- this likely means that the user intends to
                    // not only use it as an iterator, but also access the remainder using
                    // `(into_)remainder`. For now, just give a help message in this case.
                    // TODO: give a suggestion that replaces this:
                    // ```
                    // let chunk_iter = bytes.chunks_exact(CHUNK_SIZE);
                    // let remainder_chunk = chunk_iter.remainder();
                    // for chunk in chunk_iter {
                    //     /* ... */
                    // }
                    // ```
                    // with this:
                    // ```
                    // let chunk_iter = bytes.as_chunks::<CHUNK_SIZE>();
                    // let remainder_chunk = chunk_iter.1;
                    // for chunk in chunk_iter.0.iter() {
                    //     /* ... */
                    // }
                    // ```

                    diag.span_help(call_span, format!("consider using `{as_chunks}` instead"));

                    // Try to extract the variable name to provide a more helpful note
                    if let PatKind::Binding(_, _, ident, _) = let_stmt.pat.kind {
                        diag.note(format!(
                            "you can access the chunks using `{ident}.0.iter()`, and the remainder using `{ident}.1`"
                        ));
                    }
                } else {
                    diag.span_suggestion(
                        call_span,
                        "consider using `as_chunks` instead",
                        format!("{as_chunks}.0.iter()"),
                        applicability,
                    );
                }
            },
        );
    }
}
