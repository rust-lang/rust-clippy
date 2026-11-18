use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::snippet_with_applicability;
use rustc_attr_ir::lang_items::LangItem;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::ITER_NTH_ZERO;

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>, arg: &hir::Expr<'_>) {
    let def_id = cx.tcx.hir_get_parent_item(expr.hir_id).to_def_id();
    if cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
        && let Some(Constant::Int(0)) = ConstEvalCtxt::new(cx).eval_local(arg, expr.span.ctxt())
        && !cx
            .tcx
            .trait_item_of(def_id)
            .is_some_and(|trait_item| cx.tcx.is_lang_item(trait_item, LangItem::IteratorNext))
    {
        let mut app = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            ITER_NTH_ZERO,
            expr.span,
            "called `.nth(0)` on a `std::iter::Iterator`, when `.next()` is equivalent",
            "try calling `.next()` instead of `.nth(0)`",
            format!("{}.next()", snippet_with_applicability(cx, recv.span, "..", &mut app)),
            app,
        );
    }
}
