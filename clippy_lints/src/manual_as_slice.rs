use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef as _;
use rustc_attr_ir::LangItem;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects if a full range slice reference is used instead of using the `.as_slice()` method.
    ///
    /// ### Why is this bad?
    ///
    /// Using the `some_value.as_slice()` method is more explicit than using `&some_value[..]`
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
    #[clippy::version = "1.100.0"]
    pub MANUAL_AS_SLICE,
    pedantic,
    "Use as slice instead of borrow full range."
}

declare_lint_pass!(ManualAsSlice => [MANUAL_AS_SLICE]);

impl LateLintPass<'_> for ManualAsSlice {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !expr.span.from_expansion()
            && let ExprKind::AddrOf(_, mutability, borrow) = expr.kind
            && let ExprKind::Index(value, index, index_span) = borrow.kind
            && !matches!(value.kind, ExprKind::Array([]))
            && cx
                .typeck_results()
                .expr_ty_adjusted(index)
                .is_lang_item(cx, LangItem::RangeFull)
        {
            match cx.typeck_results().expr_ty(value).kind() {
                ty::Array(_, _) | ty::Slice(_) => {},
                ty::Ref(_, t, _) if let ty::Array(_, _) | ty::Slice(_) = t.kind() => {},
                ty::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Vec, adt.did()) => {},
                _ => return,
            }

            span_lint_and_then(cx, MANUAL_AS_SLICE, expr.span, "using a full range slice", |diag| {
                let sugg_tail = match mutability {
                    Mutability::Not => ".as_slice()",
                    Mutability::Mut => ".as_mut_slice()",
                };

                diag.multipart_suggestion(
                    format!("use `{sugg_tail} instead`"),
                    vec![
                        (expr.span.until(borrow.span), String::new()),
                        (index_span, sugg_tail.to_string()),
                    ],
                    Applicability::MachineApplicable,
                );
            });
        }
    }
}
