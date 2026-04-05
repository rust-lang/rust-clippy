use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs;
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for patterns like `unsafe { ptr1.offset_from(ptr2) } as usize` and suggests
    /// replacing them with `unsafe { ptr1.offset_from_unsigned(ptr2) }`.
    ///
    /// ### Why is this bad?
    /// `offset_from_unsigned` specifically mentions the offset needing to be unsigned as
    /// its additional safety precondition. Using it makes the code's intent clearer and
    /// avoids the manual cast.
    ///
    /// ### Example
    /// ```rust,no_run
    /// # let (ptr1, ptr2) = (std::ptr::null::<u8>(), std::ptr::null::<u8>());
    /// unsafe {
    ///     let _ = ptr2.offset_from(ptr1) as usize;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,no_run
    /// # let (ptr1, ptr2) = (std::ptr::null::<u8>(), std::ptr::null::<u8>());
    /// unsafe {
    ///     let _ = ptr2.offset_from_unsigned(ptr1);
    /// }
    /// ```
    #[clippy::version = "1.87.0"]
    pub MANUAL_OFFSET_FROM_UNSIGNED,
    complexity,
    "suggests using `offset_from_unsigned` instead of `offset_from(...) as usize`"
}

impl_lint_pass!(ManualOffsetFromUnsigned => [MANUAL_OFFSET_FROM_UNSIGNED]);

#[derive(Default)]
pub struct ManualOffsetFromUnsigned {
    pub msrv: Msrv,
}

impl<'tcx> LateLintPass<'tcx> for ManualOffsetFromUnsigned {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !self.msrv.meets(cx, msrvs::PTR_OFFSET_FROM_UNSIGNED) {
            return;
        }

        if let ExprKind::Cast(cast_expr, cast_ty) = expr.kind
            && let cast_to_ty = cx.typeck_results().node_type(cast_ty.hir_id)
            && cast_to_ty.is_usize()
        {
            let inner_expr = peel_blocks(cast_expr);

            if let ExprKind::MethodCall(path, receiver, [arg], _) = inner_expr.kind
                && let method_name = path.ident.name.as_str()
                && (method_name == "offset_from" || method_name == "byte_offset_from")
            {
                let receiver_ty = cx.typeck_results().expr_ty(receiver);

                let is_ptr = matches!(receiver_ty.kind(), ty::RawPtr(..));
                let is_non_null = receiver_ty
                    .ty_adt_def()
                    .is_some_and(|adt| cx.tcx.is_diagnostic_item(sym::NonNull, adt.did()));

                if is_ptr || is_non_null {
                    let mut app = Applicability::MaybeIncorrect;
                    let receiver_snip = snippet_with_applicability(cx, receiver.span, "..", &mut app);
                    let arg_snip = snippet_with_applicability(cx, arg.span, "..", &mut app);

                    let msg = format!("manual conversion from `{method_name}` to `usize`");
                    let sugg = format!("{receiver_snip}.{method_name}_unsigned({arg_snip})");

                    span_lint_and_sugg(cx, MANUAL_OFFSET_FROM_UNSIGNED, expr.span, msg, "use", sugg, app);
                }
            }
        }
    }
}

fn peel_blocks<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    match expr.kind {
        ExprKind::Block(block, _) => {
            if let Some(final_expr) = block.expr {
                peel_blocks(final_expr)
            } else if block.stmts.len() == 1
                && let rustc_hir::StmtKind::Expr(stmt_expr) = block.stmts[0].kind
            {
                peel_blocks(stmt_expr)
            } else {
                expr
            }
        },
        _ => expr,
    }
}
