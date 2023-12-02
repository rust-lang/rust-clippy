use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_opt;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// replace accessing array with an `idx`` by a `get(idx)`.
    ///
    /// ### Why is this bad?
    /// it's easier to locate error where you access array when
    /// out of boundary; and you can write your error handle to
    /// solve this problem.
    ///
    /// ### Example
    /// ```no_run
    /// let numbers = &[4, 7, 3];
        /// let my_number = numbers[5];
    /// ```
    /// Use instead:
    /// ```no_run
    /// let numbers = &[4, 7, 3];
    /// let Some(my_number) = numbers.get(5) else {
    ///     panic!();
    /// };
    /// ```
    #[clippy::version = "1.76.0"]
    pub MIGHT_PANIC,
    style,
    "use `get()` function if you're not sure whether index is legal"
}

declare_lint_pass!(MightPanic => [MIGHT_PANIC]);

impl LateLintPass<'_> for MightPanic {
    fn check_stmt<'tcx>(&mut self, cx: &LateContext<'tcx>, stmt: &hir::Stmt<'tcx>) {
        // pattern matching an `let` stmt satisfy form like `let x: ty = arr[idx];`
        if let hir::StmtKind::Local(hir::Local {
            pat,
            ty,
            init,
            els: _,
            hir_id: _,
            span,
            source: _,
        }) = &stmt.kind
            && let hir::PatKind::Binding(_, _, ident, _) = pat.kind
            && let Some(expr) = init
            && let hir::ExprKind::Index(arr, idx, _) = &expr.kind
            && let hir::ExprKind::Path(_) = arr.kind
            && let hir::ExprKind::Lit(spanned) = idx.kind
            && let LitKind::Int(v, _) = spanned.node
        {
            // get type info string by stmt.local.ty.span
            // may not have an explict type, it doesn't matter
            let mut ty_str = String::new();
            if let Some(hir::Ty {
                hir_id: _,
                kind: _,
                span: ty_span,
            }) = ty
                && let Some(snip) = snippet_opt(cx, *ty_span)
            {
                ty_str = snip;
            }
            let span = *span;
            span_lint_and_then(
                cx,
                MIGHT_PANIC,
                span,
                "use `get()` function if you're not sure whether index is legal",
                |diag| {
                    if let Some(snip) = snippet_opt(cx, arr.span) {
                        // format sugg string
                        let sugg = if ty_str.is_empty() {
                            format!("let Some({ident}) = {snip}.get({v}) else {{ panic!() }};")
                        } else {
                            format!("let Some({ident}): Option<&{ty_str}> = {snip}.get({v}) else {{ panic!() }};")
                        };
                        diag.span_suggestion(span, "Try", sugg, Applicability::MachineApplicable);
                    }
                },
            );
        }
    }
}
