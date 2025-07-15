use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{SpanlessEq, is_integer_literal};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::source_map::Spanned;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `x.get(x.len() - 1)` instead of
    /// `x.last()`.
    ///
    /// ### Why is this bad?
    /// Using `x.last()` is easier to read and has the same
    /// result.
    ///
    /// Note that using `x[x.len() - 1]` is semantically different from
    /// `x.last()`.  Indexing into the array will panic on out-of-bounds
    /// accesses, while `x.get()` and `x.last()` will return `None`.
    ///
    /// There is another lint (get_unwrap) that covers the case of using
    /// `x.get(index).unwrap()` instead of `x[index]`.
    ///
    /// ### Example
    /// ```no_run
    /// let x = vec![2, 3, 5];
    /// let last_element = x.get(x.len() - 1);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let x = vec![2, 3, 5];
    /// let last_element = x.last();
    /// ```
    #[clippy::version = "1.37.0"]
    pub GET_LAST_WITH_LEN,
    complexity,
    "Using `x.get(x.len() - 1)` when `x.last()` is correct and simpler"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, arg: &Expr<'_>) {
    // Argument to "get" is a subtraction
    if let ExprKind::Binary(
        Spanned {
            node: BinOpKind::Sub, ..
        },
        lhs,
        rhs,
    ) = arg.kind

        // LHS of subtraction is "x.len()"
        && let ExprKind::MethodCall(lhs_path, lhs_recv, [], _) = &lhs.kind
        && lhs_path.ident.name == sym::len

        // RHS of subtraction is 1
        && is_integer_literal(rhs, 1)

        // check that recv == lhs_recv `recv.get(lhs_recv.len() - 1)`
        && SpanlessEq::new(cx).eq_expr(recv, lhs_recv)
        && !recv.can_have_side_effects()
    {
        let method = match cx.typeck_results().expr_ty_adjusted(recv).peel_refs().kind() {
            ty::Adt(def, _) if cx.tcx.is_diagnostic_item(sym::VecDeque, def.did()) => "back",
            ty::Slice(_) => "last",
            _ => return,
        };

        let mut applicability = Applicability::MachineApplicable;
        let recv_snippet = snippet_with_applicability(cx, recv.span, "_", &mut applicability);

        span_lint_and_sugg(
            cx,
            GET_LAST_WITH_LEN,
            expr.span,
            format!("accessing last element with `{recv_snippet}.get({recv_snippet}.len() - 1)`"),
            "try",
            format!("{recv_snippet}.{method}()"),
            applicability,
        );
    }
}
