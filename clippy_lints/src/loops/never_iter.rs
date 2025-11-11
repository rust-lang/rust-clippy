use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use clippy_utils::visitors::{Descend, for_each_expr_without_closures};
use rustc_errors::Applicability;
use rustc_hir::{Block, Destination, Expr, ExprKind, HirId, InlineAsm, InlineAsmOperand, Node, Stmt, StmtKind};
use rustc_lint::LateContext;
use rustc_span::{BytePos, Span, sym};
use std::iter::once;
use std::ops::ControlFlow;
use clippy_utils::ty::is_type_lang_item;

use crate::loops::never_loop::{NeverLoopResult, all_spans_after_expr, combine_seq, combine_seq_many, never_loop_block, never_loop_expr};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for iterator methods with closures that always diverge.
    ///
    /// ### Why is this bad?
    /// The iterator method will only process the first element before diverging,
    /// which is often not the intended behavior.
    ///
    /// ### Example
    /// ```no_run
    /// fn diverge() -> ! {
    ///     panic!();
    /// }
    ///
    /// [0, 1].into_iter().for_each(|x| diverge()); // Only calls diverge once
    /// ```
    #[clippy::version = "1.75.0"]
    pub NEVER_ITER,
    correctness,
    "iterator methods with closures that always diverge"
}

pub fn check_iterator_diverge<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
    if let ExprKind::MethodCall(method_name, _receiver, args, _) = expr.kind {
        if is_iterator_reduction_method(method_name.ident.name) {
            if let [arg] = args {
                if let ExprKind::Closure(closure) = arg.kind {
                    let mut local_labels = Vec::new();
                    let closure_body = cx.tcx.hir().body(closure.body);
                    let diverges_in_closure = never_loop_block(cx, closure_body.value, &mut local_labels, expr.hir_id);
                    
                    if let NeverLoopResult::Diverging { .. } = diverges_in_closure {
                        span_lint_and_then(
                            cx,
                            NEVER_ITER,
                            expr.span,
                            "this iterator method never processes more than the first element",
                            |diag| {
                                let method_snippet = snippet(cx, expr.span, "..");
                                diag.span_help(
                                    expr.span,
                                    "this method will only process the first element due to divergence in the closure"
                                );
                            }
                        );
                    }
                }
            }
        }
    }
}

fn is_iterator_reduction_method(method_name: rustc_span::Symbol) -> bool {
    matches!(
        method_name,
        sym::for_each | sym::try_for_each | sym::fold | sym::reduce |
        sym::all | sym::any | sym::find | sym::find_map | sym::position | sym::rposition
    )
}