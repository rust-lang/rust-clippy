use clippy_utils::{
    diagnostics::span_lint_and_then,
    is_from_proc_macro, is_path_diagnostic_item,
    msrvs::{self, Msrv},
    ty::is_type_diagnostic_item,
    visitors::for_each_expr,
};
use rustc_hir::{ExprKind, FnRetTy, ItemKind, OwnerNode, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{lint::in_external_macro, ty};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::sym;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of `Ok` in a `let...else` statement in a function that returns
    /// `Result`.
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
    /// // If you want to explicitly ignore the contents of the `Err` variant:
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

        if let StmtKind::Local(local) = stmt.kind
            && let Some(els) = local.els
            && let OwnerNode::Item(item) = cx.tcx.hir().owner(cx.tcx.hir().get_parent_item(stmt.hir_id))
            && let ItemKind::Fn(sig, _, _) = item.kind
            && let FnRetTy::Return(ret_ty) = sig.decl.output
            && is_path_diagnostic_item(cx, ret_ty, sym::Result)
            // Only lint if we return from it
            && for_each_expr(els, |expr| {
                if matches!(expr.kind, ExprKind::Ret(..)) {
                    return ControlFlow::Break(());
                }

                ControlFlow::Continue(())
            })
            .is_some()
        {
            let spans = {
                let mut spans = vec![];
                local.pat.walk_always(|pat| {
                    let ty = cx.typeck_results().pat_ty(pat);
                    if is_type_diagnostic_item(cx, ty, sym::Result)
                        && let ty::Adt(_, substs) = ty.kind()
                        && let [_, err_ty] = substs.as_slice()
                        && let Some(err_ty) = err_ty.as_type()
                        && let Some(err_def) = err_ty.ty_adt_def()
                        && err_def.all_fields().count() != 0
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
                span_lint_and_then(
                    cx,
                    LET_ELSE_ON_RESULT_OK,
                    span,
                    "usage of `let...else` on `Ok`",
                    |diag| {
                        diag.note("this will ignore the contents of the `Err` variant");
                        diag.help("consider using a `match` instead, or propagating it to the caller");
                    }
                );
            }
        }
    }
    extract_msrv_attr!(LateContext);
}
