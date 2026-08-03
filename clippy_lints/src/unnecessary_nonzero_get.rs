use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::Msrv;
use clippy_utils::{is_from_proc_macro, span_contains_comment, sym};
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `NonZero::get` calls that are immediately followed by a method which
    /// `NonZero` provides itself, with the same return type.
    ///
    /// ### Why is this bad?
    /// The `get` call adds nothing but noise, as the method could be called on the
    /// `NonZero` value directly.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::num::NonZero;
    /// # let nz = NonZero::new(1u32).unwrap();
    /// let _ = nz.get().leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::num::NonZero;
    /// # let nz = NonZero::new(1u32).unwrap();
    /// let _ = nz.leading_zeros();
    /// ```
    #[clippy::version = "1.99.0"]
    pub UNNECESSARY_NONZERO_GET,
    complexity,
    "calling `NonZero::get` before a method `NonZero` provides itself"
}

impl_lint_pass!(UnnecessaryNonzeroGet => [UNNECESSARY_NONZERO_GET]);

pub struct UnnecessaryNonzeroGet {
    msrv: Msrv,
}

impl UnnecessaryNonzeroGet {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryNonzeroGet {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        // `<recv>.get().<method>()`
        if let ExprKind::MethodCall(method, get_call, [], _) = expr.kind
            && let ExprKind::MethodCall(get, recv, [], get_span) = get_call.kind
            && get.ident.name == sym::get
            // Both calls must resolve to inherent methods. A trait method of the same name would
            // resolve differently once the receiver becomes a `NonZero`, changing the meaning.
            // Inherent impls can only be written for local types, so an inherent `get` on
            // `NonZero` is necessarily `NonZero::get`.
            && let Some(get_did) = cx.typeck_results().type_dependent_def_id(get_call.hir_id)
            && cx.tcx.trait_of_assoc(get_did).is_none()
            && let Some(method_did) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && cx.tcx.trait_of_assoc(method_did).is_none()
            // The `get` receiver is a `NonZero<_>`
            && let nz_ty = cx.typeck_results().expr_ty_adjusted(recv).peel_refs()
            && let ty::Adt(adt, _) = nz_ty.kind()
            && cx.tcx.is_diagnostic_item(sym::NonZero, adt.did())
            // ...which has an inherent method of the same name returning the very same type, so
            // that dropping the `get` leaves the expression's type unchanged. Methods that return
            // a `NonZero` where the primitive returns a plain integer (`bit_width`, `count_ones`)
            // deliberately do not match: rewriting those would only move the `get`, not remove it.
            && let ret_ty = cx.typeck_results().expr_ty(expr)
            && has_matching_method(cx, adt.did(), nz_ty, method.ident.name, ret_ty, self.msrv)
            // Removing `.get()` means editing the span between the receiver and the method call,
            // which is only meaningful when both are written out in the same context.
            && !expr.span.from_expansion()
            && recv.span.eq_ctxt(expr.span)
            && get_call.span.eq_ctxt(expr.span)
            && recv.span.hi() <= get_call.span.hi()
            && !is_from_proc_macro(cx, expr)
        {
            // Covers `.get()` including the dot and any whitespace before it
            let removal_span = get_call.span.with_lo(recv.span.hi());
            // Do not mark this machine-applicable: removing the span could also remove comments
            // that appear between the receiver and `.get()`.
            let applicability = if span_contains_comment(cx, removal_span) {
                Applicability::MaybeIncorrect
            } else {
                Applicability::MachineApplicable
            };
            span_lint_and_then(
                cx,
                UNNECESSARY_NONZERO_GET,
                get_span,
                format!("unnecessary `get` before `{}`", method.ident.name),
                |diag| {
                    diag.span_suggestion_verbose(removal_span, "remove this", "", applicability);
                },
            );
        }
    }
}

/// Checks whether `nz_ty` has an inherent method `name` taking nothing but `self` and returning
/// exactly `ret_ty`, which is stable under `msrv`.
fn has_matching_method<'tcx>(
    cx: &LateContext<'tcx>,
    nonzero_did: DefId,
    nz_ty: Ty<'tcx>,
    name: Symbol,
    ret_ty: Ty<'tcx>,
    msrv: Msrv,
) -> bool {
    cx.tcx.inherent_impls(nonzero_did).iter().any(|&impl_did| {
        // The integer methods live in concrete `impl NonZero<u32>`-style blocks, so their
        // signatures need no instantiation. Restricting to the impl matching the receiver also
        // keeps signed-only methods off unsigned `NonZero`s and vice versa.
        cx.tcx.type_of(impl_did).instantiate_identity().skip_norm_wip() == nz_ty
            && cx
                .tcx
                .associated_items(impl_did)
                .filter_by_name_unhygienic(name)
                .any(|item| {
                    item.is_fn() && {
                        let sig = cx.tcx.fn_sig(item.def_id).instantiate_identity().skip_binder();
                        sig.inputs().len() == 1 && sig.output() == ret_ty && msrv.is_stable(cx, item.def_id)
                    }
                })
    })
}
