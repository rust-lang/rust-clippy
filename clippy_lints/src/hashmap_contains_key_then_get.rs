use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual key check in HashMap/BTreeMap and following unwrap/expect
    ///
    /// ### Why is this bad?
    /// It is less readable, can be written in an idiomatic way,
    /// and can slightly affect performance, since the hash is calculated twice
    /// ### Example
    /// ```rust,no_run
    /// use std::collections::HashMap;
    ///
    /// let map: HashMap<i32,i32> = HashMap::from_iter([(0,1)]);
    /// let key = 0;
    /// if map.contains_key(&key) {
    ///     let value = map.get(&key).unwrap();
    ///     // ...
    /// }
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// use std::collections::HashMap;
    ///
    /// let map: HashMap<i32,i32> = HashMap::from_iter([(0,1)]);
    /// let key = 0;
    /// if let Some(value) = map.get(&key) {
    ///     // ...
    /// }
    /// ```
    #[clippy::version = "1.102.0"]
    pub HASHMAP_CONTAINS_KEY_THEN_GET,
    perf,
    "unnecessary `contains_key` check before extracting associated value"
}

declare_lint_pass!(HashmapContainsKeyThenGet => [HASHMAP_CONTAINS_KEY_THEN_GET]);

const WARNING_MESSAGE: &str = "manual check for key before extracting its value from a map";
const UNWRAP_LIKE_METHODS: [rustc_span::symbol::Symbol; 5] = [
    sym::unwrap,
    sym::unwrap_or_default,
    sym::unwrap_or,
    sym::expect,
    sym::unwrap_or_else,
];

impl<'tcx> LateLintPass<'tcx> for HashmapContainsKeyThenGet {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::If(cond_expr, body, _) = expr.kind
            && let ExprKind::MethodCall(contains_path, reciever_expr, [key_expr, ..], _) = cond_expr.kind
            && let Some(sym::HashMap | sym::BTreeMap) = cx
                .typeck_results()
                .expr_ty_adjusted(reciever_expr)
                .peel_refs()
                .opt_diag_name(cx)
            && contains_path.ident.name == sym::contains_key
            && let ExprKind::Block(body, _) = body.kind
            && let &[stmt, ..] = body.stmts
            && let StmtKind::Let(let_stmt) = stmt.kind
            && let Some(init_expr) = let_stmt.init
            && let ExprKind::MethodCall(unwrap_path, unwrap_reciever, ..) = init_expr.kind
            && UNWRAP_LIKE_METHODS.contains(&unwrap_path.ident.name)
            && let unwrap_reciever_cleaned = skip_till_get(unwrap_reciever)
            && let ExprKind::MethodCall(get_path, get_reciever, [get_arg], ..) = unwrap_reciever_cleaned.kind
            && get_path.ident.name == sym::get
            && eq_expr_value(cx, expr.span.ctxt(), key_expr, get_arg)
            && eq_expr_value(cx, expr.span.ctxt(), reciever_expr, get_reciever)
        {
            let full_span = Span::new(expr.span.lo(), let_stmt.span.hi(), expr.span.ctxt(), None);
            let var_name = snippet(cx, let_stmt.pat.span, "..");
            let span_without_unwrap = Span::new(
                init_expr.span.lo(),
                unwrap_reciever.span.hi(),
                init_expr.span.ctxt(),
                None,
            );
            let assignment_snip = snippet(cx, span_without_unwrap, "..");
            let sugg = format!("if let Some({var_name}) = {assignment_snip} {{");
            span_lint_and_sugg(
                cx,
                HASHMAP_CONTAINS_KEY_THEN_GET,
                full_span,
                WARNING_MESSAGE,
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

fn skip_till_get<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    let mut expr = expr;
    while let ExprKind::MethodCall(method_name, reciever_expr, ..) = expr.kind
        && method_name.ident.name != sym::get
    {
        expr = reciever_expr;
    }
    expr
}
