use clippy_utils::diagnostics::span_lint_and_note;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;
use rustc_span::source_map::Spanned;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to [`splitn`]
    /// (https://doc.rust-lang.org/std/primitive.str.html#method.splitn) and
    /// related functions with either zero or one splits.
    ///
    /// ### Why is this bad?
    /// These calls don't actually split the value and are
    /// likely to be intended as a different number.
    ///
    /// ### Example
    /// ```no_run
    /// # let s = "";
    /// for x in s.splitn(1, ":") {
    ///     // ..
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let s = "";
    /// for x in s.splitn(2, ":") {
    ///     // ..
    /// }
    /// ```
    #[clippy::version = "1.54.0"]
    pub SUSPICIOUS_SPLITN,
    correctness,
    "checks for `.splitn(0, ..)` and `.splitn(1, ..)`"
}

pub(super) fn check(cx: &LateContext<'_>, method_name: Symbol, expr: &Expr<'_>, self_arg: &Expr<'_>, count: u128) {
    if count <= 1
        && let Some(call_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(call_id)
        && cx.tcx.impl_trait_ref(impl_id).is_none()
        && let self_ty = cx.tcx.type_of(impl_id).instantiate_identity()
        && (self_ty.is_slice() || self_ty.is_str())
    {
        // Ignore empty slice and string literals when used with a literal count.
        if matches!(self_arg.kind, ExprKind::Array([]))
            || matches!(self_arg.kind, ExprKind::Lit(Spanned { node: LitKind::Str(s, _), .. }) if s.is_empty())
        {
            return;
        }

        let (msg, note_msg) = if count == 0 {
            (
                format!("`{method_name}` called with `0` splits"),
                "the resulting iterator will always return `None`",
            )
        } else {
            (
                format!("`{method_name}` called with `1` split"),
                if self_ty.is_slice() {
                    "the resulting iterator will always return the entire slice followed by `None`"
                } else {
                    "the resulting iterator will always return the entire string followed by `None`"
                },
            )
        };

        span_lint_and_note(cx, SUSPICIOUS_SPLITN, expr.span, msg, None, note_msg);
    }
}
