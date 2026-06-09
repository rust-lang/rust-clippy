use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for tuples or arrays constructed and immediately destructured.
    ///
    /// ### Why is this bad?
    /// It creates unnecessary intermediate objects and can be more cleanly expressed
    /// using a `let` chain.
    ///
    /// ### Example
    /// ```no_run
    /// if let (Some(x), Ok(y)) = (x, y) {
    /// 
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// if let Some(x) = x && let Ok(y) = y {
    ///
    /// }
    /// ```
    #[clippy::version = "1.98.0"]
    pub TUPLE_LET_CHAIN,
    complexity,
    "tuple or array constructed and immediately destructured"
}

declare_lint_pass!(TupleLetChain => [TUPLE_LET_CHAIN]);

impl<'tcx> LateLintPass<'tcx> for TupleLetChain {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let ExprKind::Let(let_expr) = expr.kind else { return };

        let (pat_elements, expr_elements) = match (&let_expr.pat.kind, &let_expr.init.kind) {
            (PatKind::Tuple(p_els, _), ExprKind::Tup(e_els)) => (p_els, e_els),
            (PatKind::Slice(p_els, None, _), ExprKind::Array(e_els)) => (p_els, e_els),
            _ => return,
        };

        if pat_elements.len() != expr_elements.len() {
            return;
        }

        // Check if any binding in the pattern is used in subsequent init expressions.
        // This prevents incorrect refactoring where a later init depends on a binding from the pattern.
        let mut is_shadowed = false;
        for (i, pat) in pat_elements.iter().enumerate() {
            pat.each_binding(|_, _, _, bound_ident| {
                for expr in &expr_elements[i + 1..] {
                    if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind {
                        if path.segments.last().unwrap().ident.name == bound_ident.name {
                            is_shadowed = true;
                        }
                    }
                }
            });
        }

        if !is_shadowed {
            let mut sugg = String::new();
            for (i, (pat, init)) in pat_elements.iter().zip(expr_elements.iter()).enumerate() {
                let pat_snip = snippet(cx, pat.span, "..");
                let init_snip = snippet(cx, init.span, "..");
                
                if i > 0 {
                    sugg.push_str(" && let ");
                } else {
                    sugg.push_str("let ");
                }
                sugg.push_str(&format!("{pat_snip} = {init_snip}"));
            }

            span_lint_and_sugg(
                cx,
                TUPLE_LET_CHAIN,
                expr.span,
                "this tuple or array is constructed and immediately destructured",
                "consider using a `let` chain instead",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}