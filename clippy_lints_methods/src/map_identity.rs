use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{is_expr_untyped_identity_function, is_trait_method, path_to_local};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::{self as hir, Node, PatKind};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for instances of `map(f)` where `f` is the identity function.
    ///
    /// ### Why is this bad?
    /// It can be written more concisely without the call to `map`.
    ///
    /// ### Example
    /// ```no_run
    /// let x = [1, 2, 3];
    /// let y: Vec<_> = x.iter().map(|x| x).map(|x| 2*x).collect();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x = [1, 2, 3];
    /// let y: Vec<_> = x.iter().map(|x| 2*x).collect();
    /// ```
    #[clippy::version = "1.47.0"]
    pub MAP_IDENTITY,
    complexity,
    "using iterator.map(|x| x)"
}

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &hir::Expr<'_>,
    caller: &hir::Expr<'_>,
    map_arg: &hir::Expr<'_>,
    name: Symbol,
    _map_span: Span,
) {
    let caller_ty = cx.typeck_results().expr_ty(caller);

    if (is_trait_method(cx, expr, sym::Iterator)
        || is_type_diagnostic_item(cx, caller_ty, sym::Result)
        || is_type_diagnostic_item(cx, caller_ty, sym::Option))
        && is_expr_untyped_identity_function(cx, map_arg)
        && let Some(sugg_span) = expr.span.trim_start(caller.span)
    {
        // If the result of `.map(identity)` is used as a mutable reference,
        // the caller must not be an immutable binding.
        if cx.typeck_results().expr_ty_adjusted(expr).is_mutable_ptr()
            && let Some(hir_id) = path_to_local(caller)
            && let Node::Pat(pat) = cx.tcx.hir_node(hir_id)
            && !matches!(pat.kind, PatKind::Binding(BindingMode::MUT, ..))
        {
            return;
        }

        span_lint_and_sugg(
            cx,
            MAP_IDENTITY,
            sugg_span,
            "unnecessary map of the identity function",
            format!("remove the call to `{name}`"),
            String::new(),
            Applicability::MachineApplicable,
        );
    }
}
