use clippy_utils::diagnostics::span_lint;
use clippy_utils::macros::HirNode;
use rustc_hir::{Expr, ExprKind, QPath, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `any` or `all` where the return values are ignored.
    ///
    /// Ignoring the return value of `any` or `all` is suspicious and may indicate a mistake.
    /// The returned value is usually intended to be used as a condition.
    /// If `any` or `all` is called solely for their side effects on the iterator or their `FnMut` argument,
    /// it's generally better to consume the iterator using a plain `for` loop.
    ///
    /// ### Example
    /// ```no_run
    /// let mut days_without_accident = 0;
    /// (0..).all(|day| {
    ///     if day % 5 == 0 {
    ///         return false
    ///     }
    ///
    ///     days_without_accident += 1;
    ///     true
    /// });
    /// ```

    #[clippy::version = "1.82.0"]
    pub UNUSED_ANY_ALL,
    suspicious,
    "unused result of `Iterator::any` or `Iterator::all`"
}

#[derive(Default)]
pub struct UnusedAnyAll;

impl_lint_pass!(UnusedAnyAll => [UNUSED_ANY_ALL]);

impl<'tcx> LateLintPass<'tcx> for UnusedAnyAll {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let sym_any = sym::any;
        let sym_all = sym::all;
        let sym_iter = sym::Iterator;

        let ExprKind::Block(block, _label) = &expr.kind else {
            return;
        };

        for statement in block.stmts {
            let StmtKind::Semi(semi) = statement.kind else { continue };

            let method_name = match &semi.kind {
                ExprKind::MethodCall(path, expr, _args, _span)
                    if path.ident.name == sym_any
                        || path.ident.name == sym_all && clippy_utils::is_trait_method(cx, expr, sym::Iterator) =>
                {
                    path.ident.name
                },
                ExprKind::Call(path, _args) => match path.kind {
                    ExprKind::Path(QPath::Resolved(_, path))
                        if path
                            .segments
                            .first()
                            .map(|s| s.ident.name)
                            .zip(path.segments.last().map(|s| s.ident.name))
                            == Some((sym_iter, sym_all)) =>
                    {
                        sym_all
                    },
                    ExprKind::Path(QPath::Resolved(_, path))
                        if path
                            .segments
                            .first()
                            .map(|s| s.ident.name)
                            .zip(path.segments.last().map(|s| s.ident.name))
                            == Some((sym_iter, sym_any)) =>
                    {
                        sym_any
                    },
                    _ => continue,
                },
                _ => continue,
            };

            span_lint(
                cx,
                UNUSED_ANY_ALL,
                semi.span(),
                format!("unused result of `Iterator::{method_name}`"),
            );
        }
    }
}
