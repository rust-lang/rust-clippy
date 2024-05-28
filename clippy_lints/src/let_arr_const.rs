use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
use clippy_utils::visitors::is_const_evaluatable;
use hir::{BindingMode, ExprKind, PatKind, Stmt, StmtKind};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for defining of read-only arrays on stack.
    ///
    /// ### Why is this bad?
    /// `let array` puts array on the stack. As as intended, the compiler will
    /// initialize the array directly on the stack, which might make the
    /// generated binary bigger and slower.
    ///
    /// A read-only array should be defined as a `static` item or used a trick
    /// to made it into `.rodata` section of the compiled program. The use of
    /// the trick (`*&<array literal>`) will make rustc static-promote the
    /// array literal. Which means that the array now lives in the read-only
    /// section of the generated binary. The downside of the trick is that
    /// it is non-ergonomic and may not be very clear to readers of the code.
    ///
    /// ### Known problems
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a: [u32; N] = [1, 3, 5, ...];
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let a: [u32; N] = *&[1, 3, 5, ...];
    /// // or
    /// static A: [u32; N] = [1, 3, 5, ...];
    /// ```
    #[clippy::version = "1.80.0"]
    pub LET_ARR_CONST,
    perf,
    "declare a read-only array on stack"
}

declare_lint_pass!(LetArrConst => [LET_ARR_CONST]);

// FIXME: See also LARGE_CONST_ARRAYS, LARGE_STACK_ARRAYS.
impl<'tcx> LateLintPass<'tcx> for LetArrConst {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'_>) {
        // let pat: ty = init;
        if let StmtKind::Let(local) = stmt.kind
            && let PatKind::Binding(BindingMode::NONE, _, _name, None) = local.pat.kind
            && let Some(init) = local.init
            && !init.span.from_expansion()
        {
            // LLVM optimizes the load of 16 byte as a single `mov`.
            // Bigger values make more `mov` instructions generated.
            // While changing code as this lint suggests, it becomes
            // a single load (`lea`) of an address in `.rodata`.
            const STACK_THRESHOLD: u64 = 16;
            // lint only `if size_of(init) > STACK_THRESHOLD`
            let ty = cx.typeck_results().expr_ty(init);
            if let Ok(layout) = cx.tcx.layout_of(cx.param_env.and(ty))
                && let size = layout.layout.size()
                && size.bytes() <= STACK_THRESHOLD
            {
                return;
            }

            let mut applicability = Applicability::MachineApplicable;
            let lang_items = cx.tcx.lang_items();
            let Some(copy_id) = lang_items.copy_trait() else {
                return;
            };

            // if init is [<Copy type>; N]
            if let ExprKind::Repeat(..) = init.kind {
                // `init` optimized as the same code.
                return;
            }

            // if init is [1, 2, 3, ...]
            let ExprKind::Array(items @ [ref expr, ..]) = init.kind else {
                return;
            };

            let ty = cx.typeck_results().expr_ty(expr);

            if implements_trait(cx, ty, copy_id, &[]) && items.iter().all(|expr| is_const_evaluatable(cx, expr)) {
                let msg = "using `static` to push the array to read-only section of program or try";
                let snippet = snippet_with_applicability(cx, init.span, "_", &mut applicability);
                let sugg = format!("*&{snippet}");
                span_lint_and_sugg(
                    cx,
                    LET_ARR_CONST,
                    init.span,
                    "declaring a read-only array on the stack",
                    msg,
                    sugg,
                    applicability,
                );
            }
        }
    }
}
