use rustc::hir;
use rustc::hir::{Expr, Local};
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::ty;
use crate::utils::{match_qpath, match_type, match_type_parameter, snippet, span_lint_and_sugg, walk_ptrs_ty, get_type_parameter};
use crate::utils::paths;

/// **What it does:**
/// Checks for usage of `Rc<String>` or `Rc<Vec<T>>`.
///
/// **Why is this bad?**
/// Using `Rc<str>` or `Rc<[T]>` is more efficient and easy to construct with
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
/// let good_ref: Rc<[usize]> = vec!(1, 2, 3).into();
/// ```
declare_clippy_lint! {
    pub USE_SHARED_FROM_SLICE,
    nursery,
    "constructing reference-counted type from `Vec` or `String`"
}

#[derive(Copy, Clone)]
pub struct Pass;

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(USE_SHARED_FROM_SLICE)
    }
}

/// If the given `expr` is constructing an `Rc` or `Arc` containing a `Vec` or
/// `String`, output a suggestion to fix accordingly.
fn check_rc_construction<'a, 'tcx>(cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
    let expr_ty = walk_ptrs_ty(cx.tables.expr_ty(expr));

    // Check for expressions with the type `Rc<Vec<T>>`.
    if_chain! {
        // Check if this expression is constructing an `Rc` or `Arc`.
        let is_rc = match_type(cx, expr_ty, &paths::RC);
        let is_arc = match_type(cx, expr_ty, &paths::ARC);
        if is_rc || is_arc;

        // Check if the `Rc` or `Arc` is constructed with `Vec` or `String`.
        if let ty::TyAdt(_, subst) = expr_ty.sty;
        let arg_type = subst.type_at(0);
        let arg_is_vec = match_type(cx, arg_type, &paths::VEC);
        let arg_is_string = match_type(cx, arg_type, &paths::STRING);
        if arg_is_vec || arg_is_string;

        // Get the argument, to use for writing out the lint message.
        if let hir::ExprCall(_, ref args) = expr.node;
        if let Some(arg) = args.get(0);

        then {
            if arg_is_vec {
                let msg = "avoid constructing reference-counted type from Vec; convert from slice instead";
                let help = "use";
                let argument = snippet(cx, arg.span.source_callsite(), "..");
                let sugg = format!("{}.into()", argument);
                span_lint_and_sugg(cx, USE_SHARED_FROM_SLICE, expr.span, &msg, help, sugg);
            } else if arg_is_string {
                let msg = "avoid constructing reference-counted type from String; convert from slice instead";
                let help = "use";
                let argument = snippet(cx, arg.span.source_callsite(), "..");
                let sugg = format!("{}.as_str().into()", argument);
                span_lint_and_sugg(cx, USE_SHARED_FROM_SLICE, expr.span, &msg, help, sugg);
            }
        }
    }
}

/// Check a type declaration to lint, such as in
///
///     let x: Rc<String> = Rc::new(some_string)
///
/// If `ty`, the declared type, is an `Rc` or `Arc` containing a `Vec` or
/// `String` then output a suggestion to change it.
fn check_rc_type<'a, 'tcx>(cx: &LateContext<'a, 'tcx>, ty: &hir::Ty) {
    match ty.node {
        hir::TyPath(ref qpath) => {
            let matches_rc = match_qpath(qpath, &paths::RC);
            let matches_arc = match_qpath(qpath, &paths::ARC);
            if matches_rc || matches_arc {
                let has_vec = match_type_parameter(cx, qpath, &paths::VEC);
                let has_string = match_type_parameter(cx, qpath, &paths::STRING);
                // Keep the type for making suggestions later.
                let constructor = if matches_arc { "Arc" } else { "Rc" };
                if_chain! {
                    if has_vec;
                    // In the case we have something like `Rc<Vec<usize>>`, get the inner parameter
                    // type out from the parameter type of the `Rc`; so in this example, get the
                    // type `usize`. Use this to suggest using the type `Rc<[usize]>` instead.
                    let mut vec_ty = get_type_parameter(qpath).expect("");
                    if let hir::TyPath(ref vec_qpath) = vec_ty.node;
                    if let Some(param_ty) = get_type_parameter(&vec_qpath);
                    then {
                        let msg = "use slice instead of `Vec` in reference-counted type";
                        let help = "use";
                        let sugg = format!("{}<[{}]>", constructor, snippet(cx, param_ty.span.source_callsite(), ".."));
                        span_lint_and_sugg(cx, USE_SHARED_FROM_SLICE, ty.span, msg, help, sugg);
                    }
                }
                if has_string {
                    //ty.node = TyPath(hir::Resolved(None, P()))
                    let msg = "use slice instead of `String` in reference-counted type";
                    let help = "use";
                    let sugg = format!("{}<str>", constructor);
                    span_lint_and_sugg(cx, USE_SHARED_FROM_SLICE, ty.span, msg, help, sugg);
                }
            }
        },
        _ => {},
    }
}

impl <'a, 'tcx> LateLintPass<'a, 'tcx> for Pass {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        check_rc_construction(cx, expr);
    }

    fn check_local(&mut self, cx: &LateContext<'a, 'tcx>, local: &Local) {
        if let Some(ref ty) = local.ty {
            check_rc_type(cx, ty);
        }
    }
}
