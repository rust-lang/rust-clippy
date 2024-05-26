use clippy_utils::diagnostics::span_lint_and_help;
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
    /// A read-only array should be declared as a `static` item or used tricks
    /// to made it into `.rodata` section of the compiled file.
    ///
    /// ### Known problems
    /// `let array` puts array on the stack which might make the generated binary file
    /// bigger and slower.
    ///
    /// ### Example
    /// ```no_run
    /// let a = [0; 64];
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let a = *&[0; 64];
    /// // or
    /// static A: [u32; 64] = [0; 64];
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

            let generic_msg = "using `static` to push the array to read-only section of program";
            let report_static = || {
                span_lint_and_help(
                    cx,
                    LET_ARR_CONST,
                    local.span,
                    "declaring a read-only array on the stack",
                    None,
                    generic_msg,
                );
            };

            let mut should = false;
            // if init is [<Copy type>; 42]
            if let ExprKind::Repeat(value, _length) = init.kind {
                let ty = cx.typeck_results().expr_ty(value);
                if !implements_trait(cx, ty, copy_id, &[]) {
                    report_static();
                    return;
                }
                should = true;
            }
            // if init is [1, 2, 3, 4]
            if let ExprKind::Array(items @ [ref expr, ..]) = init.kind
                && let ty = cx.typeck_results().expr_ty(expr)
                && implements_trait(cx, ty, copy_id, &[])
                && items.iter().all(|expr| is_const_evaluatable(cx, expr))
            {
                should = true;
            }

            if should {
                let snippet = snippet_with_applicability(cx, init.span, "_", &mut applicability);
                let msg = format!("{generic_msg} or try: `*&{snippet}`");
                span_lint_and_help(
                    cx,
                    LET_ARR_CONST,
                    init.span,
                    "declaring a read-only array on the stack",
                    None,
                    msg,
                );
            }
        }
    }
}
