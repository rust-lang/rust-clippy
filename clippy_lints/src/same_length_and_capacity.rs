use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use clippy_utils::{SpanlessEq, sym};
use rustc_hir::{Expr, ExprKind, LangItem, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym as rustc_sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for usages of Vec::from_raw_parts and String::from_raw_parts
    /// where the same expression is used for the length and the capacity.
    ///
    /// ### Why is this bad?
    ///
    /// If the same expression is being passed for the length and
    /// capacity, it is most likely a semantic error. In the case of a
    /// Vec, for example, the only way to end up with one that has
    /// the same length and capacity is by going through a boxed slice,
    /// e.g. Box::from(some_vec), which shrinks the capacity to match
    /// the length.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// let mut original: Vec::<i32> = Vec::with_capacity(20);
    /// original.extend([1, 2, 3, 4, 5]);
    ///
    /// let (ptr, mut len, cap) = original.into_raw_parts();
    ///
    /// // Pretend we added three more integers:
    /// len = 8;
    ///
    /// // But I forgot the capacity was separate from the length:
    /// let reconstructed = unsafe { Vec::from_raw_parts(ptr, len, len) };
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// // Correction to the last line of the given example code:
    /// let reconstructed = unsafe { Vec::from_raw_parts(ptr, len, cap) };
    /// ```
    #[clippy::version = "1.91.0"]
    pub SAME_LENGTH_AND_CAPACITY,
    pedantic,
    "`from_raw_parts` with same length and capacity"
}
declare_lint_pass!(SameLengthAndCapacity => [SAME_LENGTH_AND_CAPACITY]);

impl<'tcx> LateLintPass<'tcx> for SameLengthAndCapacity {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(path_expr, args) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, fn_path)) = path_expr.kind
            && is_type_diagnostic_item(cx, cx.typeck_results().node_type(ty.hir_id), rustc_sym::Vec)
            && fn_path.ident.name == sym::from_raw_parts
            && SpanlessEq::new(cx).eq_expr(&args[1], &args[2])
        {
            span_lint_and_help(
                cx,
                SAME_LENGTH_AND_CAPACITY,
                expr.span,
                "usage of `Vec::from_raw_parts` with the same expression for length and capacity",
                None,
                "if the length and capacity are the same, you most likely went through a boxed slice; consider reconstructing the `Vec` using a `Box` instead, e.g. `Box::from(slice::from_raw_parts(...)).into_vec()`",
            );
        } else if let ExprKind::Call(path_expr, args) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, fn_path)) = path_expr.kind
            && is_type_lang_item(cx, cx.typeck_results().node_type(ty.hir_id), LangItem::String)
            && fn_path.ident.name == sym::from_raw_parts
            && SpanlessEq::new(cx).eq_expr(&args[1], &args[2])
        {
            span_lint_and_help(
                cx,
                SAME_LENGTH_AND_CAPACITY,
                expr.span,
                "usage of `String::from_raw_parts` with the same expression for length and capacity",
                None,
                "if the length and capacity are the same, you most likely went through a boxed `str`; consider reconstructing the `String` using `String::from` instead, e.g. `String::from(str::from_utf8_unchecked(slice::from_raw_parts(...)))`",
            );
        }
    }
}
