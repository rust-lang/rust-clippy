use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{is_lang_item_or_ctor, is_trait_method};
use hir::{LangItem, OwnerNode};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of `iter.nth(0)`.
    ///
    /// ### Why is this bad?
    /// `iter.next()` is equivalent to
    /// `iter.nth(0)`, as they both consume the next element,
    ///  but is more readable.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::collections::HashSet;
    /// # let mut s = HashSet::new();
    /// # s.insert(1);
    /// let x = s.iter().nth(0);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # use std::collections::HashSet;
    /// # let mut s = HashSet::new();
    /// # s.insert(1);
    /// let x = s.iter().next();
    /// ```
    #[clippy::version = "1.42.0"]
    pub ITER_NTH_ZERO,
    style,
    "replace `iter.nth(0)` with `iter.next()`"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>, arg: &hir::Expr<'_>) {
    if let OwnerNode::Item(item) = cx.tcx.hir_owner_node(cx.tcx.hir_get_parent_item(expr.hir_id))
        && let def_id = item.owner_id.to_def_id()
        && is_trait_method(cx, expr, sym::Iterator)
        && let Some(Constant::Int(0)) = ConstEvalCtxt::new(cx).eval(arg)
        && !is_lang_item_or_ctor(cx, def_id, LangItem::IteratorNext)
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
