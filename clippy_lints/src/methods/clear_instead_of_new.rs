use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath, UnOp};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::CLEAR_INSTEAD_OF_NEW;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
    // At this point, mod.rs has already checked:
    // - expr.span.from_expansion()
    // - expr.kind is ExprKind::Assign

    if let ExprKind::Assign(target, value, _) = expr.kind
        && let ExprKind::Call(func, args) = value.kind
        && args.is_empty()
        && let ExprKind::Path(QPath::TypeRelative(ty_path, method)) = func.kind
        && method.ident.name == sym::new
    {
        let ty = cx.typeck_results().node_type(ty_path.hir_id);

        if matches!(
            ty.peel_refs().opt_diag_name(cx),
            Some(
                sym::Vec
                    | sym::HashMap
                    | sym::HashSet
                    | sym::VecDeque
                    | sym::BTreeMap
                    | sym::BTreeSet
                    | sym::BinaryHeap
                    | sym::LinkedList
            )
        ) {
            let mut peeled = target;
            while let ExprKind::Unary(UnOp::Deref, inner) = peeled.kind {
                peeled = inner;
            }

            span_lint_and_sugg(
                cx,
                CLEAR_INSTEAD_OF_NEW,
                expr.span,
                "assigning a new empty collection",
                "consider using `.clear()` instead",
                format!("{}.clear()", snippet(cx, peeled.span, "..")),
                Applicability::MaybeIncorrect,
            );
        }
    }
}
