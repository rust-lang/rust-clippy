use clippy_utils::{
    diagnostics::span_lint_and_note,
    is_from_proc_macro, is_lang_item_or_ctor,
    msrvs::{self, Msrv},
};
use rustc_hir::{LangItem, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of `Ok` in a `let...else` statement.
    ///
    /// ### Why is this bad?
    /// This will ignore the contents of the `Err` variant, which is generally unintended and not
    /// desired. Alternatively, it can be propagated to the caller.
    ///
    /// ### Example
    /// ```rust
    /// # fn foo() -> Result<(), ()> {
    /// #    Ok(())
    /// # }
    /// let Ok(foo) = foo() else {
    ///     return;
    /// };
    /// ```
    /// Use instead:
    /// ```rust
    /// # fn foo() -> Result<(), ()> {
    /// #    Err(())
    /// # }
    /// // If you want the contents of the `Err` variant:
    /// let foo = match foo() {
    ///     Ok(foo) => foo,
    ///     Err(e) => eprintln!("{e:#?}"),
    /// };
    /// ```
    /// ```rust
    /// # fn foo() -> Result<(), ()> {
    /// #    Ok(())
    /// # }
    /// // If you want to propagate it to the caller:
    /// let foo = foo()?;
    /// # Ok::<(), ()>(())
    /// ```
    /// ```rust
    /// # fn foo() -> Result<(), ()> {
    /// #    Err(())
    /// # }
    /// // If you want to ignore the contents of the `Err` variant:
    /// let Some(foo) = foo().ok() else {
    ///     return;
    /// };
    /// ```
    #[clippy::version = "1.72.0"]
    pub LET_ELSE_ON_RESULT_OK,
    pedantic,
    "checks for usage of `Ok` in `let...else` statements"
}
impl_lint_pass!(LetElseOnResultOk => [LET_ELSE_ON_RESULT_OK]);

pub struct LetElseOnResultOk {
    msrv: Msrv,
}

impl LetElseOnResultOk {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl<'tcx> LateLintPass<'tcx> for LetElseOnResultOk {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &Stmt<'tcx>) {
        if !self.msrv.meets(msrvs::LET_ELSE) || in_external_macro(cx.sess(), stmt.span) {
            return;
        }

        if let StmtKind::Local(local) = stmt.kind && let Some(els) = local.els {
            let spans = {
                let mut spans = vec![];
                local.pat.walk_always(|pat| {
                    if let PatKind::TupleStruct(qpath, _, _) = pat.kind
                        && let Some(def_id) = cx.qpath_res(&qpath, pat.hir_id).opt_def_id()
                        && is_lang_item_or_ctor(cx, def_id, LangItem::ResultOk)
                    {
                        spans.push(pat.span);
                    }
                });
                spans
            };

            if !spans.is_empty() && is_from_proc_macro(cx, els) {
                return;
            };

            for span in spans {
                span_lint_and_note(
                    cx,
                    LET_ELSE_ON_RESULT_OK,
                    span,
                    "usage of `let...else` on `Err`",
                    None,
                    "consider handling the `Err` variant gracefully or propagating it to the caller",
                );
            }
        }
    }
    extract_msrv_attr!(LateContext);
}
