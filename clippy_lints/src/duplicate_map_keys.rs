use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{last_path_segment, SpanlessEq};
use rustc_hir::ExprKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// When two items are inserted into a `HashMap` with the same key,
    /// the second item will overwrite the first item.
    ///
    /// ### Why is this bad?
    /// This can lead to data loss.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::collections::HashMap;
    /// let example = HashMap::from([(5, 1), (5, 2)]);
    /// ```
    #[clippy::version = "1.79.0"]
    pub DUPLICATE_MAP_KEYS,
    suspicious,
    "`HashMap` with duplicate keys loses data"
}

declare_lint_pass!(DuplicateMapKeys => [DUPLICATE_MAP_KEYS]);

impl<'tcx> LateLintPass<'tcx> for DuplicateMapKeys {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if has_hash_collision(cx, expr).is_some_and(|v| !v.is_empty()) {
            span_lint_and_note(
                cx,
                DUPLICATE_MAP_KEYS,
                expr.span,
                "this `HashMap` uses one key for multiple items and will lose data",
                None,
                "consider using a different keys for all items",
            );
        }
    }
}

// TODO: Also check for different sources of hash collisions
// TODO: Maybe other types of hash maps should be checked as well?
fn has_hash_collision<'a>(
    cx: &LateContext<'_>,
    expr: &'a rustc_hir::Expr<'_>,
) -> Option<Vec<(rustc_hir::Expr<'a>, rustc_hir::Expr<'a>)>> {
    // If the expression is a call to `HashMap::from`, check if the keys are the same
    if let ExprKind::Call(func, args) = &expr.kind
        // First check for HashMap::from
        && let ty = cx.typeck_results().expr_ty(expr)
        && is_type_diagnostic_item(cx, ty, sym::HashMap)
        && let ExprKind::Path(func_path) = func.kind
        && last_path_segment(&func_path).ident.name == sym::from
        // There should be exactly one argument to HashMap::from
        && args.len() == 1
        // which should be an array
        && let ExprKind::Array(args) = &args[0].kind
    // Then check the keys
    {
        // Put all keys in a vector
        let mut keys = Vec::new();

        for arg in *args {
            //
            if let ExprKind::Tup(args) = &arg.kind
                && !args.is_empty()
            // && let ExprKind::Lit(lit) = args[0].kind
            {
                keys.push(args[0]);
            }
        }

        // Check if there are any duplicate keys
        let mut duplicate = Vec::new();
        let mut spannless_eq = SpanlessEq::new(cx);
        for i in 0..keys.len() {
            for j in i + 1..keys.len() {
                if spannless_eq.eq_expr(&keys[i], &keys[j]) {
                    duplicate.push((keys[i], keys[j]));
                }
            }
        }
        return Some(duplicate);
    }
    None
}
