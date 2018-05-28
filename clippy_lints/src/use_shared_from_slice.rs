use rustc::hir::Expr;
use rustc::lint::{LateContext, LateLintPass, LintArray, LintContext, LintPass};
use rustc::ty;
use utils::{match_type, span_lint_and_sugg, walk_ptrs_ty};
use utils::paths;

/// **What it does:**
/// Checks for usage of `Rc<String>` or `Rc<Vec<T>>`.
///
/// **Why is this bad?**
/// Using a `Rc<str>` or `Rc<[T]>` is more efficient and easy to construct with
/// `into()`.
///
/// **Known problems:**
/// None.
///
/// **Example:**
///
/// ```rust
/// use std::rc::Rc;
///
/// // Bad
/// let bad_ref: Rc<Vec<usize>> = Rc::new(vec!(1, 2, 3));
///
/// // Good
/// let good_ref: Rc<[usize]> = Rc::new(vec!(1, 2, 3).into());
/// ```
declare_clippy_lint! {
    pub USE_SHARED_FROM_SLICE,
    nursery,
    "use `into()` to construct `Rc` from slice"
}

#[derive(Copy, Clone, Debug)]
pub struct Pass;

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(USE_SHARED_FROM_SLICE)
    }
}

impl <'a, 'tcx> LateLintPass<'a, 'tcx> for Pass {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        let expr_ty = walk_ptrs_ty(cx.tables.expr_ty(expr));

        // Check for expressions with the type `Rc<Vec<T>>`.
        if_chain! {
            if let ty::TyAdt(_, subst) = expr_ty.sty;
            if match_type(cx, expr_ty, &paths::RC);
            if match_type(cx, subst.type_at(1), &paths::VEC);
            then {
                cx.sess().note_without_error(&format!("{:?}", subst));
                span_lint_and_sugg(
                    cx,
                    USE_SHARED_FROM_SLICE,
                    expr.span,
                    "constructing reference-counted type from vec",
                    "consider using `into()`",
                    format!("{}", "TODO"),
                );
            }
        }

        // TODO
        // Check for expressions with the type `Rc<String>`.
        // Check for expressions with the type `Arc<String>`.
        // Check for expressions with the type `Arc<Vec<T>>`.
    }
}
