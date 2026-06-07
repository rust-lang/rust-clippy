use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::eager_or_lazy::switch_to_lazy_eval;
use clippy_utils::{get_builtin_attr, sym};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::{FnKind, Visitor, walk_body, walk_expr};
use rustc_hir::{Body, Expr, ExprKind, FnDecl};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Searches for elements marked with `#[clippy::avoid_eager_arguments]` that are being called with eagerly
    /// evaluated arguments.
    ///
    /// ### Why is this bad?
    ///
    /// If the value of an argument is expected to be rarely needed, it is usually better to evaluate that value lazily,
    /// especially if the eager evaluation involves memory allocations or other non-trivial amounts of work.
    ///
    /// ### Known Problems
    ///
    /// If the function that is called to produce the argument has side-effects, not calling it will change the
    /// semantics of the program, but this should not be relied on.
    ///
    /// The lint also cannot figure out whether the function that is being called is actually expensive to call or not.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #[clippy::avoid_eager_arguments = "The value behind `argument` may not always be used, prefer using `lazy_foo` instead"]
    /// fn foo(argument: String) {
    ///   // ...
    /// }
    ///
    /// fn lazy_foo<F>(argument: F)
    /// where
    ///     F: Fn() -> String
    /// {
    ///   // ...
    /// }
    ///
    /// foo(String::new());
    /// ```
    /// Use instead:
    /// ```rust,ignore
    ///  lazy_foo(|| String::new());
    /// ```
    ///
    /// ### Notes
    ///
    /// Library authors should provide an explanation as to why the eager evaluation is undesirable, and ideally offer
    /// and mention an alternative function that accepts an argument that can be lazily evaluated.
    #[clippy::version = "1.97.0"]
    pub EAGER_FUN_CALL,
    nursery,
    "calling a function with eagerly evaluated arguments where it is discouraged to do so"
}

declare_lint_pass!(EagerFunCall => [EAGER_FUN_CALL]);

impl<'tcx> LateLintPass<'tcx> for EagerFunCall {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'_>,
        _: &FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: LocalDefId,
    ) {
        let mut visitor = EagerFunCallVisitor { cx };
        walk_body(&mut visitor, body);
    }
}

struct EagerFunCallVisitor<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for EagerFunCallVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(func, args) = expr.kind
            && let ExprKind::Path(ref qpath) = func.kind
            && let Some(def_id) = self.cx.qpath_res(qpath, func.hir_id).opt_def_id()
        {
            #[allow(deprecated)]
            let attrs = self.cx.tcx.get_all_attrs(def_id);
            let mut lazy_attrs = get_builtin_attr(self.cx.sess(), attrs, sym::avoid_eager_arguments);
            if let Some(attr) = lazy_attrs.next() {
                let Some(message) = attr.value_str() else {
                    walk_expr(self, expr);
                    return;
                };

                let lazy_args: Vec<_> = args.iter().filter(|arg| switch_to_lazy_eval(self.cx, arg)).collect();

                if !lazy_args.is_empty() {
                    span_lint_and_note(
                        self.cx,
                        EAGER_FUN_CALL,
                        expr.span,
                        message.as_str().to_owned(),
                        Some(expr.span),
                        "function call with eagerly evaluated arguments",
                    );
                }
            }
        }
        walk_expr(self, expr);
    }
}
