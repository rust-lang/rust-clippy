use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_custom_deref_adjustment;
use clippy_utils::ty::{is_type_diagnostic_item, peel_mid_ty_refs_is_mutable};
use rustc_errors::Applicability;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `&mut Mutex::lock` calls
    ///
    /// ### Why is this bad?
    /// `Mutex::lock` is less efficient than
    /// calling `Mutex::get_mut`. In addition you also have a statically
    /// guarantee that the mutex isn't locked, instead of just a runtime
    /// guarantee.
    ///
    /// ### Example
    /// ```no_run
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut value_rc = Arc::new(Mutex::new(42_u8));
    /// let value_mutex = Arc::get_mut(&mut value_rc).unwrap();
    ///
    /// let mut value = value_mutex.lock().unwrap();
    /// *value += 1;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut value_rc = Arc::new(Mutex::new(42_u8));
    /// let value_mutex = Arc::get_mut(&mut value_rc).unwrap();
    ///
    /// let value = value_mutex.get_mut().unwrap();
    /// *value += 1;
    /// ```
    #[clippy::version = "1.49.0"]
    pub MUT_MUTEX_LOCK,
    style,
    "`&mut Mutex::lock` does unnecessary locking"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, ex: &'tcx Expr<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    if matches!(expr_custom_deref_adjustment(cx, recv), None | Some(Mutability::Mut))
        && let (_, ref_depth, Mutability::Mut) = peel_mid_ty_refs_is_mutable(cx.typeck_results().expr_ty(recv))
        && ref_depth >= 1
        && let Some(method_id) = cx.typeck_results().type_dependent_def_id(ex.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
        && is_type_diagnostic_item(cx, cx.tcx.type_of(impl_id).instantiate_identity(), sym::Mutex)
    {
        span_lint_and_sugg(
            cx,
            MUT_MUTEX_LOCK,
            name_span,
            "calling `&mut Mutex::lock` unnecessarily locks an exclusive (mutable) reference",
            "change this to",
            "get_mut".to_owned(),
            Applicability::MaybeIncorrect,
        );
    }
}
