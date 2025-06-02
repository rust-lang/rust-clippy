use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_default_equivalent_call;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
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
            if is_box_of_default(cx, lhs_ty) && is_default_call(cx, rhs) && !rhs.span.from_expansion() {
                span_lint_and_then(
                    cx,
                    DEFAULT_BOX_ASSIGNMENTS,
                    expr.span,
                    "assigning `Default::default()` to `Box<T>`",
                    |diag| {
                        let mut app = Applicability::MachineApplicable;
                        let suggestion = format!(
                            "{} = Default::default()",
                            Sugg::hir_with_applicability(cx, lhs, "_", &mut app).deref()
                        );

                        diag.note("this creates a needless allocation").span_suggestion(
                            expr.span,
                            "assign to the inner value",
                            suggestion,
                            app,
                        );
                    },
                );
            }
        }
    }
}

fn is_box_of_default<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::Adt(def, args) = ty.kind()
        && cx.tcx.is_lang_item(def.did(), LangItem::OwnedBox)
        && let Some(default_trait_id) = cx.tcx.get_diagnostic_item(sym::Default)
    {
        implements_trait(cx, args.type_at(0), default_trait_id, &[])
    } else {
        false
    }
}

fn is_default_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Call(func, _args) if is_default_equivalent_call(cx, func, Some(expr)))
}
