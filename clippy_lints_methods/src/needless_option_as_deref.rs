use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::{path_res, sym};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_hir::def::Res;
use rustc_lint::LateContext;
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for no-op uses of `Option::{as_deref, as_deref_mut}`,
    /// for example, `Option<&T>::as_deref()` returns the same type.
    ///
    /// ### Why is this bad?
    /// Redundant code and improving readability.
    ///
    /// ### Example
    /// ```no_run
    /// let a = Some(&1);
    /// let b = a.as_deref(); // goes from Option<&i32> to Option<&i32>
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let a = Some(&1);
    /// let b = a;
    /// ```
    #[clippy::version = "1.57.0"]
    pub NEEDLESS_OPTION_AS_DEREF,
    complexity,
    "no-op use of `deref` or `deref_mut` method to `Option`."
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, name: Symbol) {
    let typeck = cx.typeck_results();
    let outer_ty = typeck.expr_ty(expr);

    if is_type_diagnostic_item(cx, outer_ty, sym::Option) && outer_ty == typeck.expr_ty(recv) {
        if name == sym::as_deref_mut && recv.is_syntactic_place_expr() {
            let Res::Local(binding_id) = path_res(cx, recv) else {
                return;
            };

            if local_used_after_expr(cx, binding_id, recv) {
                return;
            }
        }

        span_lint_and_sugg(
            cx,
            NEEDLESS_OPTION_AS_DEREF,
            expr.span,
            "derefed type is same as origin",
            "try",
            recv.span.get_source_text(cx).unwrap().to_owned(),
            Applicability::MachineApplicable,
        );
    }
}
