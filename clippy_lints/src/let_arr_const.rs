use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
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

impl LateLintPass<'_> for LetArrConst {
    // should lint on `let array` which non-mut and:
    // if let repeat:
    // if let expr: copy
    // todo: repair clippy::author
    fn check_stmt(&mut self, cx: &LateContext<'_>, stmt: &Stmt<'_>) {
        if let StmtKind::Let(ref local) = stmt.kind
            && let PatKind::Binding(BindingMode::NONE, _, _name, None) = local.pat.kind
            && let Some(ref init) = local.init
            && !init.span.from_expansion()
        {
            let mut applicability = Applicability::MachineApplicable;
            let lang_items = cx.tcx.lang_items();
            let Some(copy_id) = lang_items.copy_trait() else {
                return;
            };
            // `let arr = [<Copy type>; 42];
            let mut should = false;
            if let ExprKind::Repeat(ref value, _length) = init.kind {
                let ty = cx.typeck_results().expr_ty(value);
                if !implements_trait(cx, ty, copy_id, &[]) {
                    span_lint_and_help(
                        cx,
                        LET_ARR_CONST,
                        local.span,
                        "declaring a read-only array on the stack",
                        None,
                        "using `static` to push the array to read-only section of program",
                    );
                    return;
                }
                should = true;
            }
            // `let arr = [1, 2, 3, 4];
            if let ExprKind::Array([ref expr, ..]) = init.kind
                && let ty = cx.typeck_results().expr_ty(expr)
                && implements_trait(cx, ty, copy_id, &[])
            {
                should = true;
            }

            if should {
                let snippet = snippet_with_applicability(cx, init.span, "_", &mut applicability);
                let sugg = format!("*&{snippet}");
                span_lint_and_sugg(
                    cx,
                    LET_ARR_CONST,
                    init.span,
                    "declaring a read-only array on the stack",
                    "using `static` to push the array to read-only section of program or try",
                    sugg,
                    applicability,
                );
            }
        }
    }
}
