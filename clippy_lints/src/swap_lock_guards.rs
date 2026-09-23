use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeResPath as _};
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `std::mem::swap` that swap mutable lock guards
    /// instead of the values protected by those guards.
    ///
    /// ### Why is this bad?
    /// Swapping the guards only exchanges which lock each local variable owns;
    /// it does not change either protected value.
    ///
    /// ### Example
    /// ```no_run
    /// use std::sync::Mutex;
    /// let left = Mutex::new(1);
    /// let right = Mutex::new(2);
    /// let mut left_guard = left.lock().unwrap();
    /// let mut right_guard = right.lock().unwrap();
    /// std::mem::swap(&mut left_guard, &mut right_guard);
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::sync::Mutex;
    /// # let left = Mutex::new(1);
    /// # let right = Mutex::new(2);
    /// # let mut left_guard = left.lock().unwrap();
    /// # let mut right_guard = right.lock().unwrap();
    /// std::mem::swap(&mut *left_guard, &mut *right_guard);
    /// ```
    #[clippy::version = "1.100.0"]
    pub SWAP_LOCK_GUARDS,
    suspicious,
    "swapping lock guards instead of the values protected by them"
}

declare_lint_pass!(SwapLockGuards => [SWAP_LOCK_GUARDS]);

impl<'tcx> LateLintPass<'tcx> for SwapLockGuards {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(function, [left, right]) = expr.kind
            && function.basic_res().is_diag_item(cx, sym::mem_swap)
            && let Some(left_inner) = mutable_borrowed_expr(left)
            && let Some(right_inner) = mutable_borrowed_expr(right)
            && is_mutable_guard(cx, cx.typeck_results().expr_ty(left_inner))
        {
            span_lint_and_then(
                cx,
                SWAP_LOCK_GUARDS,
                expr.span,
                "swapping the guards does not swap the protected values",
                |diag| {
                    diag.multipart_suggestion(
                        "dereference the guards to swap the protected values",
                        vec![
                            (left_inner.span.shrink_to_lo(), "*".to_owned()),
                            (right_inner.span.shrink_to_lo(), "*".to_owned()),
                        ],
                        Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}

fn mutable_borrowed_expr<'a, 'tcx>(expr: &'a Expr<'tcx>) -> Option<&'a Expr<'tcx>> {
    if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, inner) = expr.kind {
        Some(inner)
    } else {
        None
    }
}

fn is_mutable_guard(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    matches!(
        cx.tcx.get_diagnostic_name(adt.did()),
        Some(sym::MutexGuard | sym::RwLockWriteGuard | sym::RefCellRefMut)
    )
}
