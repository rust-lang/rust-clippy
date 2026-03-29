use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;

use super::EXTEND_WITH_DRAIN;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, arg: &Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();

    // Each of these containers has:
    // * A method `fn append(&mut self, other: &mut Self)`.
    // * A method `fn drain(&mut self, /* maybe a range parameter too */)`.
    for (container_type_sym, expect_drain_range_argument) in
        [(sym::Vec, true), (sym::VecDeque, true), (sym::BinaryHeap, false)]
    {
        if ty.is_diag_item(cx, container_type_sym)
        // Check that `extend()`'s argument is `drain()`
        && let ExprKind::MethodCall(src_method, drain_vec, drain_args, _) = &arg.kind
        && src_method.ident.name == sym::drain
        // Check that the receiver of `drain()` is a collection of the same type
        && let src_ty = cx.typeck_results().expr_ty(drain_vec)
        && src_ty.peel_refs().is_diag_item(cx, container_type_sym)
        // Check that the drain range (if there is one) is full, not partial
        && drain_args_are_full_range(cx, drain_args, expect_drain_range_argument)
        {
            let mut applicability = Applicability::MachineApplicable;
            span_lint_and_sugg(
                cx,
                EXTEND_WITH_DRAIN,
                expr.span,
                format!(
                    "use of `extend` instead of `append` for moving \
                    the full contents of a second `{container_type_sym}`"
                ),
                "try",
                format!(
                    "{}.append({}{})",
                    snippet_with_applicability(cx, recv.span, "..", &mut applicability),
                    if src_ty.is_mutable_ptr() { "" } else { "&mut " },
                    snippet_with_applicability(cx, drain_vec.span, "..", &mut applicability)
                ),
                applicability,
            );
        }
    }
}

/// Check for the correct count of arguments to a `drain()` call, and, if a range is expected,
/// that the range is the full range `..`.
fn drain_args_are_full_range(cx: &LateContext<'_>, args: &[Expr<'_>], expect_drain_range_argument: bool) -> bool {
    match (expect_drain_range_argument, args) {
        (false, []) => true,
        (true, [drain_arg]) => {
            let src_ty_range = cx.typeck_results().expr_ty(drain_arg).peel_refs();
            src_ty_range.is_lang_item(cx, LangItem::RangeFull)
        },
        (_, _) => false,
    }
}
