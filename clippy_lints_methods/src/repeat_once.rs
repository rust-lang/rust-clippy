use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_lang_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, LangItem};
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.repeat(1)` and suggest the following method for each types.
    /// - `.to_string()` for `str`
    /// - `.clone()` for `String`
    /// - `.to_vec()` for `slice`
    ///
    /// The lint will evaluate constant expressions and values as arguments of `.repeat(..)` and emit a message if
    /// they are equivalent to `1`. (Related discussion in [rust-clippy#7306](https://github.com/rust-lang/rust-clippy/issues/7306))
    ///
    /// ### Why is this bad?
    /// For example, `String.repeat(1)` is equivalent to `.clone()`. If cloning
    /// the string is the intention behind this, `clone()` should be used.
    ///
    /// ### Example
    /// ```no_run
    /// fn main() {
    ///     let x = String::from("hello world").repeat(1);
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn main() {
    ///     let x = String::from("hello world").clone();
    /// }
    /// ```
    #[clippy::version = "1.47.0"]
    pub REPEAT_ONCE,
    complexity,
    "using `.repeat(1)` instead of `String.clone()`, `str.to_string()` or `slice.to_vec()` "
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    recv: &'tcx Expr<'_>,
    repeat_arg: &'tcx Expr<'_>,
) {
    if ConstEvalCtxt::new(cx).eval(repeat_arg) == Some(Constant::Int(1)) {
        let ty = cx.typeck_results().expr_ty(recv).peel_refs();
        if ty.is_str() {
            span_lint_and_sugg(
                cx,
                REPEAT_ONCE,
                expr.span,
                "calling `repeat(1)` on str",
                "consider using `.to_string()` instead",
                format!("{}.to_string()", snippet(cx, recv.span, r#""...""#)),
                Applicability::MachineApplicable,
            );
        } else if ty.builtin_index().is_some() {
            span_lint_and_sugg(
                cx,
                REPEAT_ONCE,
                expr.span,
                "calling `repeat(1)` on slice",
                "consider using `.to_vec()` instead",
                format!("{}.to_vec()", snippet(cx, recv.span, r#""...""#)),
                Applicability::MachineApplicable,
            );
        } else if is_type_lang_item(cx, ty, LangItem::String) {
            span_lint_and_sugg(
                cx,
                REPEAT_ONCE,
                expr.span,
                "calling `repeat(1)` on a string literal",
                "consider using `.clone()` instead",
                format!("{}.clone()", snippet(cx, recv.span, r#""...""#)),
                Applicability::MachineApplicable,
            );
        }
    }
}
