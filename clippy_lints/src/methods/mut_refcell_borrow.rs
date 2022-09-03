use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::{expr_custom_deref_adjustment, match_def_path, paths};
use rustc_errors::Applicability;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;

use super::MUT_REFCELL_BORROW;

//TODO calls to RefCell's `Clone`-impl could be replaced by `RefCell::new(foo.get_mut().clone())`
//to circumvent the runtime-check

fn emit_replace(cx: &LateContext<'_>, name_span: Span) {
    span_lint_and_help(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::replace()` unnecessarily performs a runtime-check that can never fail",
        None,
        "use `.get_mut()` to get a mutable reference to the value, and replace the value using `std::mem::replace()` or direct assignment",
    );
}

fn emit_replace_with(cx: &LateContext<'_>, name_span: Span) {
    span_lint_and_help(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::replace_with()` unnecessarily performs a runtime-check that can never fail",
        None,
        "use `.get_mut()` to get a mutable reference to the value, and replace the value using `std::mem::replace()` or direct assignment",
    );
}

fn emit_borrow(cx: &LateContext<'_>, name_span: Span) {
    // This is not MachineApplicable as `borrow` returns a `Ref` while `get_mut` returns a
    // `&mut T`, and we don't check surrounding types
    span_lint_and_sugg(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::borrow()` unnecessarily performs a runtime-check that can never fail",
        "change this to",
        "get_mut".to_owned(),
        Applicability::MaybeIncorrect,
    );
}

fn emit_try_borrow(cx: &LateContext<'_>, name_span: Span) {
    span_lint_and_help(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::try_borrow()` unnecessarily performs a runtime-check that can never fail",
        None,
        "use `.get_mut()` instead of `.try_borrow()` to get a reference to the value; remove the error-handling",
    );
}

fn emit_borrow_mut(cx: &LateContext<'_>, name_span: Span) {
    // This is not MachineApplicable as `borrow_mut` returns a different type than `get_mut`, for
    // which we don't check
    span_lint_and_sugg(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::borrow_mut()` unnecessarily performs a runtime-check that can never fail",
        "change this to",
        "get_mut".to_owned(),
        Applicability::MaybeIncorrect,
    );
}

fn emit_try_borrow_mut(cx: &LateContext<'_>, name_span: Span) {
    span_lint_and_help(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::try_borrow_mut()` unnecessarily performs a runtime-check that can never fail",
        None,
        "use `.get_mut()` instead of `.try_borrow_mut()` to get a mutable reference to the value; remove the error-handling",
    );
}

fn emit_take(cx: &LateContext<'_>, name_span: Span) {
    span_lint_and_help(
        cx,
        MUT_REFCELL_BORROW,
        name_span,
        "calling `&mut RefCell::take()` unnecessarily performs a runtime-check that can never fail",
        None,
        "use `.get_mut()` to get a mutable reference to the value, and `std::mem::take()` to get ownership via that reference",
    );
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    ex: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'tcx>,
    name_span: Span,
    name: &'tcx str,
    arg: Option<&'tcx Expr<'tcx>>,
) {
    if matches!(expr_custom_deref_adjustment(cx, recv), None | Some(Mutability::Mut))
      && let ty::Ref(_, _, Mutability::Mut) = cx.typeck_results().expr_ty(recv).kind()
      && let Some(method_id) = cx.typeck_results().type_dependent_def_id(ex.hir_id)
      && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
      && match_def_path(cx, impl_id, &paths::REFCELL)
    {
        //TODO: Use `arg` to emit better suggestions
        match (name, arg) {
            ("replace", Some(_arg)) => emit_replace(cx, name_span),
            ("replace_with", Some(_arg)) => emit_replace_with(cx, name_span),
            ("borrow", None) => emit_borrow(cx, name_span),
            ("try_borrow", None) => emit_try_borrow(cx, name_span),
            ("borrow_mut", None) => emit_borrow_mut(cx, name_span),
            ("try_borrow_mut", None) => emit_try_borrow_mut(cx, name_span),
            ("take", None) => emit_take(cx, name_span),
            _ => unreachable!(),
        };
    }
}
