use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{MEM_TAKE, Msrv};
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{SpanlessEq, std_or_core, sym};
use rustc_errors::Applicability;
use rustc_hir::{Block, LangItem, StmtKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Symbol;

use super::{CLEAR_THEN_SHRINK, method_call};

const ACCEPTABLE_TYPES: [Symbol; 7] = [
    sym::BinaryHeap,
    sym::HashMap,
    sym::HashSet,
    sym::OsString,
    sym::PathBuf,
    sym::Vec,
    sym::VecDeque,
];

pub(super) fn check(cx: &LateContext<'_>, block: &Block<'_>, msrv: Msrv) {
    if !msrv.meets(cx, MEM_TAKE) {
        return;
    }

    let Some(std_or_core) = std_or_core(cx) else {
        return;
    };

    for [clear_stmt, shrink_stmt] in block.stmts.array_windows() {
        if let StmtKind::Semi(clear_expr) = clear_stmt.kind
            && let Some((sym::clear, clear_recv, [], ..)) = method_call(clear_expr)
            && let StmtKind::Semi(shrink_expr) = shrink_stmt.kind
            && let Some((sym::shrink_to_fit, shrink_recv, [], ..)) = method_call(shrink_expr)
            && clear_stmt.span.eq_ctxt(shrink_stmt.span)
            && !clear_stmt.span.in_external_macro(cx.tcx.sess.source_map())
            && !shrink_stmt.span.in_external_macro(cx.tcx.sess.source_map())
            && SpanlessEq::new(cx).eq_expr(clear_recv, shrink_recv)
            && !clear_recv.can_have_side_effects()
            && is_acceptable_type(cx, clear_recv)
        {
            let mut applicability = Applicability::MachineApplicable;
            let recv = snippet_with_applicability(cx, clear_recv.span, "_", &mut applicability);
            let recv_ty = cx.typeck_results().expr_ty(clear_recv);
            let sugg = if matches!(recv_ty.kind(), ty::Ref(..)) {
                format!("{std_or_core}::mem::take({recv});")
            } else {
                format!("{std_or_core}::mem::take(&mut {recv});")
            };

            span_lint_and_sugg(
                cx,
                CLEAR_THEN_SHRINK,
                clear_stmt.span.to(shrink_stmt.span),
                "calling `clear` and then `shrink_to_fit`",
                "consider using",
                sugg,
                applicability,
            );
        }
    }
}

fn is_acceptable_type(cx: &LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    ty.is_lang_item(cx, LangItem::String) || ACCEPTABLE_TYPES.iter().any(|&sym| ty.is_diag_item(cx, sym))
}
