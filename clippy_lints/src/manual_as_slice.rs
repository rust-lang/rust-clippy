use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, Mutability, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyKind;
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects if a full range slice reference is used instead of using the `.as_slice()` method.
    ///
    /// ### Why is this bad?
    ///
    /// Using the `some_value.as_slice()` method is more explicit then using `&some_value[..]`
    ///
    /// ### Example
    /// ```no_run
    /// let array: [u8; 4] = [0; 4];
    /// let slice = &array[..];
    /// ```
    /// Use instead:
    /// ```no_run
    /// let array: [u8; 4] = [0; 4];
    /// let slice = array.as_slice();
    /// ```
    #[clippy::version = "1.88.0"]
    pub MANUAL_AS_SLICE,
    nursery,
    "Use as slice instead of borrow full range."
}
declare_lint_pass!(ManualAsSlice => [MANUAL_AS_SLICE]);

impl LateLintPass<'_> for ManualAsSlice {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::AddrOf(_, mutability, borrow) = expr.kind
            && let ExprKind::Index(value, index, index_span) = borrow.kind
            && let ExprKind::Struct(qpath, _, _) = index.kind
            && let QPath::LangItem(LangItem::RangeFull, _) = qpath
        {
            let sugg_tail = match mutability {
                Mutability::Not => ".as_slice()",
                Mutability::Mut => ".as_mut_slice()",
            };

            let borrow_span = expr.span.until(borrow.span);
            let app = Applicability::MachineApplicable;

            match cx.typeck_results().expr_ty(value).kind() {
                TyKind::Array(_, _) | TyKind::Slice(_) => {},
                TyKind::Ref(_, t, _) if let TyKind::Array(_, _) | TyKind::Slice(_) = t.kind() => {},
                TyKind::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Vec, adt.did()) => {},
                _ => return,
            }

            span_lint_and_then(cx, MANUAL_AS_SLICE, expr.span, "using a full range slice", |diag| {
                diag.multipart_suggestion(
                    "try",
                    vec![(borrow_span, String::new()), (index_span, sugg_tail.to_string())],
                    app,
                );
            });
        }
    }
}
