use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `assert!(s.is_empty())` and `assert!(!s.is_empty())`.
    ///
    /// ### Why is this bad?
    /// `assert!` only reports the condition was false, not what the collection
    /// contained. Using `assert_eq!` / `assert_ne!` with an empty literal
    /// prints the collection contents on failure, making debugging faster.
    ///
    /// ### Example
    /// ```no_run
    /// # let items = vec![1, 2, 3];
    /// assert!(items.is_empty());
    /// assert!(!items.is_empty());
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let items = vec![1, 2, 3];
    /// assert_eq!(items, []);
    /// assert_ne!(items, []);
    /// ```
    #[clippy::version = "1.97.0"]
    pub ASSERTIONS_ON_COLLECTION_EMPTINESS,
    nursery,
    "asserting on `.is_empty()` without showing the collection contents"
}

declare_lint_pass!(AssertionsOnCollectionEmptiness => [ASSERTIONS_ON_COLLECTION_EMPTINESS]);

impl<'tcx> LateLintPass<'tcx> for AssertionsOnCollectionEmptiness {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, e)
            && matches!(
                cx.tcx.get_diagnostic_name(macro_call.def_id),
                Some(sym::assert_macro | sym::debug_assert_macro),
            )
            && let Some((condition, panic_expn)) = find_assert_args(cx, e, macro_call.expn)
            && panic_expn.is_default_message()
        {
            let (method_name, recv, is_negated) = match condition.kind {
                ExprKind::MethodCall(ms, r, [], _) => (ms.ident.name, r, false),
                ExprKind::Unary(UnOp::Not, inner) if let ExprKind::MethodCall(ms, r, [], _) = inner.kind => {
                    (ms.ident.name, r, true)
                },
                _ => return,
            };

            if method_name != sym::is_empty {
                return;
            }

            let message = if is_negated {
                "used `assert!` with a non-empty collection check"
            } else {
                "used `assert!` with an empty collection check"
            };

            let recv_ty = cx.typeck_results().expr_ty(recv);
            let empty_literal = empty_literal_for_type(cx, recv_ty);

            span_lint_and_then(
                cx,
                ASSERTIONS_ON_COLLECTION_EMPTINESS,
                macro_call.span,
                message,
                |diag| {
                    let mut app = Applicability::MachineApplicable;
                    let recv_snippet = snippet_with_context(cx, recv.span, condition.span.ctxt(), "..", &mut app).0;

                    let sugg = if is_negated {
                        format!("assert_ne!({recv_snippet}, {empty_literal})")
                    } else {
                        format!("assert_eq!({recv_snippet}, {empty_literal})")
                    };
                    diag.span_suggestion(macro_call.span, "replace with", sugg, app);
                },
            );
        }
    }
}

fn empty_literal_for_type(cx: &LateContext<'_>, ty: Ty<'_>) -> &'static str {
    if ty.is_str() || ty.peel_refs().is_str() || ty.is_lang_item(cx, LangItem::String) {
        "\"\""
    } else {
        "[]"
    }
}
