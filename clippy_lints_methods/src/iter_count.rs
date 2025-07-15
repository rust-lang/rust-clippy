use super::utils::derefs_to_slice;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::{Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of `.iter().count()`.
    ///
    /// ### Why is this bad?
    /// `.len()` is more efficient and more
    /// readable.
    ///
    /// ### Example
    /// ```no_run
    /// let some_vec = vec![0, 1, 2, 3];
    ///
    /// some_vec.iter().count();
    /// &some_vec[..].iter().count();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let some_vec = vec![0, 1, 2, 3];
    ///
    /// some_vec.len();
    /// &some_vec[..].len();
    /// ```
    #[clippy::version = "1.52.0"]
    pub ITER_COUNT,
    complexity,
    "replace `.iter().count()` with `.len()`"
}

pub(crate) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, recv: &'tcx Expr<'tcx>, iter_method: Symbol) {
    let ty = cx.typeck_results().expr_ty(recv);
    let caller_type = if derefs_to_slice(cx, recv, ty).is_some() {
        "slice"
    } else if is_type_diagnostic_item(cx, ty, sym::Vec) {
        "Vec"
    } else if is_type_diagnostic_item(cx, ty, sym::VecDeque) {
        "VecDeque"
    } else if is_type_diagnostic_item(cx, ty, sym::HashSet) {
        "HashSet"
    } else if is_type_diagnostic_item(cx, ty, sym::HashMap) {
        "HashMap"
    } else if is_type_diagnostic_item(cx, ty, sym::BTreeMap) {
        "BTreeMap"
    } else if is_type_diagnostic_item(cx, ty, sym::BTreeSet) {
        "BTreeSet"
    } else if is_type_diagnostic_item(cx, ty, sym::LinkedList) {
        "LinkedList"
    } else if is_type_diagnostic_item(cx, ty, sym::BinaryHeap) {
        "BinaryHeap"
    } else {
        return;
    };
    let mut applicability = Applicability::MachineApplicable;
    span_lint_and_sugg(
        cx,
        ITER_COUNT,
        expr.span,
        format!("called `.{iter_method}().count()` on a `{caller_type}`"),
        "try",
        format!(
            "{}.len()",
            snippet_with_applicability(cx, recv.span, "..", &mut applicability),
        ),
        applicability,
    );
}
