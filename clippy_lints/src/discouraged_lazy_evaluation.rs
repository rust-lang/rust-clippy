use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::eager_or_lazy::switch_to_eager_eval;
use clippy_utils::{get_builtin_attr, is_from_proc_macro, sym, usage};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{FnKind, Visitor, walk_body, walk_expr};
use rustc_hir::{Body, Closure, ClosureKind, Expr, ExprKind, FnDecl};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// As the counterpart to `eager_fun_call`, this lint looks for unnecessarily lazily evaluated closures used to
    /// produce arguments for functions marked with `#[clippy::optional_lazy_eval]`.
    ///
    /// ### Why is this bad?
    ///
    /// Using eager evaluation is shorter and simpler in some cases.
    ///
    /// ### Known Problems
    ///
    /// It is possible, but not recommended for `Deref` and `Index` to have side effects. Eagerly evaluating them can change the semantics of the program.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn foo(argument: String) {
    ///     // Perform some logic that only rarely involves the use of `argument`
    /// }
    ///
    /// #[clippy::optional_lazy_eval = "If `argument` does not require evaluation, prefer using `foo` instead."]
    /// fn lazy_foo<F>(argument: F)
    /// where
    ///     F: FnOnce() -> String
    /// {
    ///     /// Perform some logic and evaluate `argument` on the fly, only if it is needed.
    /// }
    ///
    /// let s = String::new("bar");
    /// lazy_foo(move || s);
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let s = String::new("bar");
    /// foo(s);
    /// ```
    #[clippy::version = "1.97.0"]
    pub DISCOURAGED_LAZY_EVALUATION,
    nursery,
    "calling a function with an unnecessary closure used to produce an argument"
}

declare_lint_pass!(DiscouragedLazyEvaluation => [DISCOURAGED_LAZY_EVALUATION]);

impl<'tcx> LateLintPass<'tcx> for DiscouragedLazyEvaluation {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'_>,
        _: &FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: LocalDefId,
    ) {
        let mut visitor = DiscouragedLazyEvaluationVisitor { cx };
        walk_body(&mut visitor, body);
    }
}

struct DiscouragedLazyEvaluationVisitor<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for DiscouragedLazyEvaluationVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(func, args) = expr.kind
            && let ExprKind::Path(ref qpath) = func.kind
            && let Some(def_id) = self.cx.qpath_res(qpath, func.hir_id).opt_def_id()
        {
            #[allow(deprecated)]
            let attrs = self.cx.tcx.get_all_attrs(def_id);
            let mut lazy_attrs = get_builtin_attr(self.cx.sess(), attrs, sym::optional_lazy_eval);
            if let Some(attr) = lazy_attrs.next() {
                let Some(message) = attr.value_str() else {
                    walk_expr(self, expr);
                    return;
                };

                let lazy_args: Vec<_> = args
                    .iter()
                    .filter(|arg| {
                        let ExprKind::Closure(&Closure {
                            body,
                            kind: ClosureKind::Closure,
                            ..
                        }) = arg.kind
                        else {
                            return false;
                        };

                        let body = self.cx.tcx.hir_body(body);
                        let body_expr = &body.value;

                        if usage::BindingUsageFinder::are_params_used(self.cx, body)
                            || is_from_proc_macro(self.cx, expr)
                        {
                            return false;
                        }

                        switch_to_eager_eval(self.cx, body_expr)
                    })
                    .collect();

                if !lazy_args.is_empty() {
                    span_lint_and_note(
                        self.cx,
                        DISCOURAGED_LAZY_EVALUATION,
                        expr.span,
                        message.as_str().to_owned(),
                        Some(expr.span),
                        "unnecessary closure used to produce a function argument",
                    );
                }
            }
        }
        walk_expr(self, expr);
    }
}
