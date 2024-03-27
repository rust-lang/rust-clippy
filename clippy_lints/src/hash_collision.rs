use clippy_utils::{diagnostics::span_lint_and_note, last_path_segment, ty::is_type_diagnostic_item};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_hir::ExprKind;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// When two items are inserted into a `HashMap` with the same key,
    /// the second item will overwrite the first item.
    /// ### Why is this bad?
    /// This can lead to data loss.
    /// ### Example
    /// ```no_run
    /// # use std::collections::HashMap;
    /// let example = HashMap::from([(5, 1), (5, 2)]);
    /// ```
    #[clippy::version = "1.79.0"]
    pub HASH_COLLISION,
    suspicious,
    "default lint description"
}

declare_lint_pass!(HashCollision => [HASH_COLLISION]);

impl<'tcx> LateLintPass<'tcx> for HashCollision {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if has_hash_collision(cx, expr) {
            span_lint_and_note(
                cx,
                HASH_COLLISION,
                expr.span,
                "This `HashMap` has a hash collision and will lose data",
                None,
                "Consider using a different keys for all items",
            )
        }
    }
}

// TODO: Also check for different sources of hash collisions
// TODO: Maybe other types of hash maps should be checked as well?
fn has_hash_collision(cx: &LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
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
            let mut literals = Vec::new();
            
            for arg in args.iter() { // 
                if let ExprKind::Tup(args) = &arg.kind 
                && args.len() > 0
                && let ExprKind::Lit(lit) = args[0].kind
                {
                    literals.push(lit.node.clone());
                }
                
        }
        // Debug: it gets here, but doesn't trigger the lint

        // Check if there are any duplicate keys
        let mut duplicate = false;
        for i in 0..literals.len() {
            for j in i + 1..literals.len() {
                if literals[i] == literals[j] {
                    duplicate = true;
                    break;
                }
            }
            if duplicate {
                break;
            }
        }
        return duplicate;
    }
    false
}
