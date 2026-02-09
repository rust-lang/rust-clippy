use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects when you reassign a collection variable with `Collection::new()`
    /// instead of just calling `.clear()` on it.
    ///
    /// ### Why is this bad?
    /// Creating a new collection discards the allocated memory of the previous
    /// collection. Using `.clear()` instead reuses the allocated memory, which
    /// can be more efficient, especially if the collection had a large capacity.
    ///
    /// ### Example
    /// ```no_run
    /// let mut vec = Vec::new();
    /// vec.push(1);
    /// vec = Vec::new();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut vec = Vec::new();
    /// vec.push(1);
    /// vec.clear();
    /// ```
    #[clippy::version = "1.XX.0"]
    pub CLEAR_INSTEAD_OF_NEW,
    perf,
    "assigning a new empty collection instead of clearing the existing one"
}

declare_lint_pass!(ClearInsteadOfNew => [CLEAR_INSTEAD_OF_NEW]);

impl<'tcx> LateLintPass<'tcx> for ClearInsteadOfNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Look for assignment expressions: x = Collection::new()
        if let ExprKind::Assign(target, value, _) = expr.kind {
            if let ExprKind::Path(QPath::Resolved(None, target_path)) = target.kind {
                if let Some(target_segment) = target_path.segments.first() {
                    if let ExprKind::Call(func, args) = value.kind {
                        if args.is_empty() && is_collection_new_call(cx, func) {
                            let var_name = target_segment.ident.as_str();

                            span_lint_and_sugg(
                                cx,
                                CLEAR_INSTEAD_OF_NEW,
                                expr.span,
                                format!("assigning a new empty collection to `{var_name}`"),
                                "consider using `.clear()` instead",
                                format!("{var_name}.clear()"),
                                Applicability::MaybeIncorrect,
                            );
                        }
                    }
                }
            }
        }
    }
}

// Check if the expression is a call to a collection's new() method
fn is_collection_new_call(cx: &LateContext<'_>, func: &Expr<'_>) -> bool {
    if let ExprKind::Path(QPath::TypeRelative(ty, method)) = func.kind
        && method.ident.name.as_str() == "new"
    {
        let ty = cx.typeck_results().node_type(ty.hir_id);
        let type_name = ty.to_string();

        // Check if it's one of the standard collections
        // Type names come in the format "std::vec::Vec<T>" or "std::collections::HashMap<K, V>"
        type_name.contains("::Vec<")
            || type_name.contains("::HashMap<")
            || type_name.contains("::HashSet<")
            || type_name.contains("::VecDeque<")
            || type_name.contains("::BTreeMap<")
            || type_name.contains("::BTreeSet<")
            || type_name.contains("::BinaryHeap<")
            || type_name.contains("::LinkedList<")
    } else {
        false
    }
}
