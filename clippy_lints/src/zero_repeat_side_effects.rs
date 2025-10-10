use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::VecArgs;
use clippy_utils::is_let_desugar;
use clippy_utils::source::{indent_of, snippet};
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{ConstArgKind, Expr, ExprKind, LetStmt, Node};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for array or vec initializations which contain an expression with side effects,
    /// but which have a repeat count of zero.
    ///
    /// ### Why is this bad?
    /// Such an initialization, despite having a repeat length of 0, will still call the inner function.
    /// This may not be obvious and as such there may be unintended side effects in code.
    ///
    /// ### Example
    /// ```no_run
    /// fn side_effect() -> i32 {
    ///     println!("side effect");
    ///     10
    /// }
    /// let a = [side_effect(); 0];
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn side_effect() -> i32 {
    ///     println!("side effect");
    ///     10
    /// }
    /// side_effect();
    /// let a: [i32; 0] = [];
    /// ```
    #[clippy::version = "1.79.0"]
    pub ZERO_REPEAT_SIDE_EFFECTS,
    suspicious,
    "usage of zero-sized initializations of arrays or vecs causing side effects"
}

declare_lint_pass!(ZeroRepeatSideEffects => [ZERO_REPEAT_SIDE_EFFECTS]);

impl LateLintPass<'_> for ZeroRepeatSideEffects {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let Some(args) = VecArgs::hir(cx, expr)
            && let VecArgs::Repeat(inner_expr, len) = args
            && let ExprKind::Lit(l) = len.kind
            && let LitKind::Int(Pu128(0), _) = l.node
        {
            inner_check(cx, expr, inner_expr, true);
        }
        // Lint only if the length is a literal zero, and not a path to any constants.
        // NOTE(@y21): When reading `[f(); LEN]`, I intuitively expect that the function is called and it
        // doesn't seem as confusing as `[f(); 0]`. It would also have false positives when eg.
        // the const item depends on `#[cfg]s` and has different values in different compilation
        // sessions).
        else if let ExprKind::Repeat(inner_expr, const_arg) = expr.kind
            && let ConstArgKind::Anon(anon_const) = const_arg.kind
            && let length_expr = cx.tcx.hir_body(anon_const.body).value
            && !length_expr.span.from_expansion()
            && let ExprKind::Lit(literal) = length_expr.kind
            && let LitKind::Int(Pu128(0), _) = literal.node
        {
            inner_check(cx, expr, inner_expr, false);
        }
    }
}

fn inner_check(cx: &LateContext<'_>, expr: &'_ Expr<'_>, inner_expr: &'_ Expr<'_>, is_vec: bool) {
    // check if expr is a call or has a call inside it
    if inner_expr.can_have_side_effects() {
        let parent_hir_node = cx.tcx.parent_hir_node(expr.hir_id);
        let return_type = cx.typeck_results().expr_ty(expr);

        let inner_expr = snippet(cx, inner_expr.span.source_callsite(), "..");
        let vec = if is_vec { "vec!" } else { "" };

        let (span, sugg) = match parent_hir_node {
            Node::LetStmt(l)
                if let Node::Stmt(stmt) = cx.tcx.parent_hir_node(l.hir_id)
                    && let Node::Block(block) = cx.tcx.parent_hir_node(stmt.hir_id)
                    && is_let_desugar(block)
                    && let Node::Expr(x) = cx.tcx.parent_hir_node(block.hir_id) =>
            {
                (
                    x.span,
                    lint_assign_expr(cx, x, l.pat.span, &inner_expr, return_type, vec),
                )
            },
            Node::LetStmt(l) => (l.span, lint_let_stmt(cx, l, &inner_expr, return_type, vec)),
            Node::Expr(x) if let ExprKind::Assign(l, _, _) = x.kind => {
                (x.span, lint_assign_expr(cx, x, l.span, &inner_expr, return_type, vec))
            },
            _ => (expr.span, format!("{{ {inner_expr}; {vec}[] as {return_type} }}")),
        };
        span_lint_and_then(
            cx,
            ZERO_REPEAT_SIDE_EFFECTS,
            span.source_callsite(),
            "expression with side effects as the initial value in a zero-sized array initializer",
            |diag| {
                diag.span_suggestion_verbose(
                    span.source_callsite(),
                    "consider performing the side effect separately",
                    sugg,
                    Applicability::Unspecified,
                );
            },
        );
    }
}

fn lint_let_stmt(
    cx: &LateContext<'_>,
    let_stmt: &LetStmt<'_>,
    inner_expr: &str,
    return_type: Ty<'_>,
    vec_str: &str,
) -> String {
    let indent = indent_of(cx, let_stmt.span.source_callsite()).unwrap_or_default();
    format!(
        "{inner_expr};\n{}let {var_name}: {return_type} = {vec_str}[];",
        " ".repeat(indent),
        var_name = snippet(cx, let_stmt.pat.span.source_callsite(), "..")
    )
}

fn lint_assign_expr(
    cx: &LateContext<'_>,
    outer_expr: &Expr<'_>,
    assign_expr_span: Span,
    inner_expr: &str,
    return_type: Ty<'_>,
    vec_str: &str,
) -> String {
    let mut parent_hir_node = cx.tcx.parent_hir_node(outer_expr.hir_id);
    if let Node::Stmt(stmt) = parent_hir_node {
        parent_hir_node = cx.tcx.parent_hir_node(stmt.hir_id);
    }
    let needs_curly = !matches!(parent_hir_node, Node::Block(_));

    let indent = indent_of(cx, outer_expr.span).unwrap_or_default();
    let indent_str_curly = " ".repeat(indent);
    let indent_str_inner = " ".repeat(indent + 4);

    format!(
        "{}{inner_expr};\n{}{var_name} = {vec_str}[] as {return_type}{}",
        if needs_curly {
            format!("{{\n{indent_str_inner}")
        } else {
            String::new()
        },
        if needs_curly {
            &indent_str_inner
        } else {
            &indent_str_curly
        },
        if needs_curly {
            format!("\n{indent_str_curly}}}")
        } else {
            String::new()
        },
        var_name = snippet(cx, assign_expr_span.source_callsite(), ".."),
    )
}
