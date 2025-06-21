use clippy_utils::diagnostics::{span_lint, span_lint_hir};
use clippy_utils::get_parent_expr;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Body, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Instance;
use rustc_session::impl_lint_pass;
use rustc_span::def_id::DefId;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions that call themselves from their body.
    ///
    /// ### Why restrict this?
    /// In Safety Critical contexts, recursive calls can lead to catastrophic
    /// crashes if they happen to overflow the stack.
    ///
    /// In most contexts, this is not an issue, so this lint is allow-by-default.
    ///
    /// ### Notes
    ///
    /// #### Triggers
    /// There are two things that trigger this lint:
    /// - Function calls from a function (or method) to itself,
    /// - Function pointer bindings from a function (or method) to itself.
    ///
    /// #### Independent of control flow
    /// This lint triggers whenever the conditions above are met, regardless of
    /// control flow and other such constructs.
    ///
    /// #### Blessing a recursive call
    /// The user may choose to bless a recursive call or binding using the
    /// attribute #[clippy::allowed_recursion]
    ///
    /// #### Indirect calls
    /// This lint **does not** detect indirect recursive calls.
    ///
    /// ### Examples
    /// This function will trigger the lint:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         // This line will trigger the lint
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// Using #[clippy::allowed_recursion] lets it pass:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         #[clippy::allowed_recursion]
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// This triggers the lint when `fibo` is bound to a function pointer
    /// inside `fibo`'s body
    /// ```no_run
    /// fn fibo(a: u32) -> u32 {
    ///     if a < 2 { a } else { (a - 2..a).map(fibo).sum() }
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub DIRECT_RECURSION,
    restriction,
    "functions shall not call themselves directly"
}

#[derive(Default)]
pub struct DirectRecursion {
    fn_id_stack: Vec<DefId>,
}

impl_lint_pass!(DirectRecursion => [DIRECT_RECURSION]);

impl<'tcx> LateLintPass<'tcx> for DirectRecursion {
    /// Whenever we enter a Body, we push its owner's `DefId` into the stack
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'_>) {
        self.fn_id_stack
            .push(cx.tcx.hir_body_owner_def_id(body.id()).to_def_id());
    }

    /// We then revert this when we exit said `Body`
    fn check_body_post(&mut self, _: &LateContext<'tcx>, _: &Body<'_>) {
        _ = self.fn_id_stack.pop();
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        match expr.kind {
            ExprKind::MethodCall(_, _, _, _) => {
                let typeck = cx.typeck_results();
                if let Some(basic_id) = typeck.type_dependent_def_id(expr.hir_id) {
                    // This finds default Trait implementations of methods
                    if self.fn_id_stack.contains(&basic_id) {
                        span_lint(cx, DIRECT_RECURSION, expr.span, "this method contains a call to itself");
                    }
                    // Whereas this finds non-default implementations
                    else if let args = typeck.node_args(expr.hir_id)
                        && let Ok(Some(fn_def)) = Instance::try_resolve(cx.tcx, cx.typing_env(), basic_id, args)
                        && let type_resolved_def_id = fn_def.def_id()
                        && self.fn_id_stack.contains(&type_resolved_def_id)
                    {
                        span_lint(cx, DIRECT_RECURSION, expr.span, "this method contains a call to itself");
                    }
                }
            },
            ExprKind::Path(QPath::TypeRelative(_, _)) => {
                // I'm still not sure this is proper.
                // It definitely finds the right `DefId`, though.
                let typeck = cx.typeck_results();
                if let Some(id) = typeck.type_dependent_def_id(expr.hir_id)
                    && let args = typeck.node_args(expr.hir_id)
                    && let Ok(Some(fn_def)) = Instance::try_resolve(cx.tcx, cx.typing_env(), id, args)
                {
                    let type_resolved_def_id = fn_def.def_id();

                    if self.fn_id_stack.contains(&type_resolved_def_id) {
                        emit_lint(cx, expr);
                    }
                }
            },
            // This branch takes care of finding bindings of function and method names
            // into fn pointers.
            ExprKind::Path(QPath::Resolved(_, path)) => {
                // Now we know that this Path is fully resolved.
                // We now must check if it points to a function or a method's definition.
                if let Res::Def(DefKind::Fn | DefKind::AssocFn, fn_path_id) = path.res
                // 1) Now we know that the path we've found is of a function or method definition.
                //
                // 2) We will now check if it corresponds to the path of a function we're inside
                // of.
                //
                // 3) Thankfully, we've kept track of the functions that surround us, in
                //`self.fn_id_stack`.
                //
                // 4) If the path that we've captured from `expr` coincides with one of the functions
                // in the stack, then we know we have a recursive loop.

                && self.fn_id_stack.contains(&fn_path_id)
                {
                    emit_lint(cx, expr);
                }
            },
            _ => {},
        }
    }
}

fn emit_lint<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
    let (node_id, span, msg) = if let Some(parent_expr) = get_parent_expr(cx, expr)
        && let ExprKind::Call(func, _) = parent_expr.kind
        && func.hir_id == expr.hir_id
    {
        (
            parent_expr.hir_id,
            parent_expr.span,
            "this function contains a call to itself",
        )
    } else {
        (expr.hir_id, expr.span, "this function creates a reference to itself")
    };
    span_lint_hir(cx, DIRECT_RECURSION, node_id, span, msg);
}
