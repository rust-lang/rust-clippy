use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_default_equivalent_call;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, GenericArgKind, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Detects assignments of `Default::default()` to a place of type `Box<T>`.
    ///
    /// ### Why is this bad?
    /// This incurs an extra heap allocation compared to assigning the boxed
    /// storage.
    ///
    /// ### Example
    /// ```no_run
    /// let mut b = Box::new(1u32);
    /// b = Default::default();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut b = Box::new(1u32);
    /// *b = Default::default();
    /// ```
    #[clippy::version = "1.89.0"]
    pub DEFAULT_BOX_ASSIGNMENTS,
    perf,
    "assigning `Default::default()` to `Box<T>` is inefficient"
}
declare_lint_pass!(DefaultBoxAssignments => [DEFAULT_BOX_ASSIGNMENTS]);

impl LateLintPass<'_> for DefaultBoxAssignments {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        if let ExprKind::Assign(lhs, rhs, _) = &expr.kind {
            let lhs_ty = cx.typeck_results().expr_ty(lhs);
            if is_box_of_default(lhs_ty, cx) && is_default_call(rhs, cx) && !rhs.span.from_expansion() {
                span_lint_and_then(
                    cx,
                    DEFAULT_BOX_ASSIGNMENTS,
                    expr.span,
                    "assigning `Default::default()` to `Box<T>`",
                    |diag| {
                        let suggestion = format!("*({}) = Default::default()", snippet(cx, lhs.span, "_"));

                        diag.note("this creates a needless allocation").span_suggestion(
                            expr.span,
                            "assign to the inner value",
                            suggestion,
                            Applicability::MaybeIncorrect,
                        );
                    },
                );
            }
        }
    }
}

fn is_box_of_default<'a>(ty: Ty<'a>, cx: &LateContext<'a>) -> bool {
    if let ty::Adt(def, args) = ty.kind()
        && cx.tcx.lang_items().get(LangItem::OwnedBox) == Some(def.did())
        && let Some(inner) = args.iter().find_map(|arg| match arg.kind() {
            GenericArgKind::Type(ty) => Some(ty),
            _ => None,
        })
    {
        cx.tcx
            .get_diagnostic_item(sym::Default)
            .is_some_and(|id| implements_trait(cx, inner, id, &[]))
    } else {
        false
    }
}

fn is_default_call(expr: &Expr<'_>, cx: &LateContext<'_>) -> bool {
    if let ExprKind::Call(func, _args) = expr.kind
        && is_default_equivalent_call(cx, func, Some(expr))
    {
        true
    } else {
        false
    }
}
