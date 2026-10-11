use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::ty::is_slice_like;
use rustc_attr_ir::LangItem;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects if the `&list[..]` syntax is used to turn an array/`Vec` into a slice.
    ///
    /// ### Why is this bad?
    ///
    /// Using the `.as_slice()` method makes the conversion more explicit.
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
			// Avoid linting `&[][..]`, because the suggestion -- `[].as_slice()` -- is too wordy.
            && !matches!(value.kind, ExprKind::Array([]))
            && cx
                .typeck_results()
                .expr_ty_adjusted(index)
                .is_lang_item(cx, LangItem::RangeFull)
            && is_slice_like(cx, cx.typeck_results().expr_ty_adjusted(value).peel_refs())
        {
            if let Some(boxed_content) = cx.typeck_results().expr_ty(value).boxed_ty()
                && boxed_content.is_slice()
            {
                return;
            }

            span_lint_and_then(cx, MANUAL_AS_SLICE, expr.span, "using a full range slice", |diag| {
                let sugg_tail = match mutability {
                    Mutability::Not => ".as_slice()",
                    Mutability::Mut => ".as_mut_slice()",
                };

                diag.multipart_suggestion(
                    format!("use `{sugg_tail}` instead"),
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
