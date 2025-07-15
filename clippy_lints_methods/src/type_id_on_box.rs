use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::{Adjust, Adjustment};
use rustc_middle::ty::print::with_forced_trimmed_paths;
use rustc_middle::ty::{self, ExistentialPredicate, Ty};
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Looks for calls to `.type_id()` on a `Box<dyn _>`.
    ///
    /// ### Why is this bad?
    /// This almost certainly does not do what the user expects and can lead to subtle bugs.
    /// Calling `.type_id()` on a `Box<dyn Trait>` returns a fixed `TypeId` of the `Box` itself,
    /// rather than returning the `TypeId` of the underlying type behind the trait object.
    ///
    /// For `Box<dyn Any>` specifically (and trait objects that have `Any` as its supertrait),
    /// this lint will provide a suggestion, which is to dereference the receiver explicitly
    /// to go from `Box<dyn Any>` to `dyn Any`.
    /// This makes sure that `.type_id()` resolves to a dynamic call on the trait object
    /// and not on the box.
    ///
    /// If the fixed `TypeId` of the `Box` is the intended behavior, it's better to be explicit about it
    /// and write `TypeId::of::<Box<dyn Trait>>()`:
    /// this makes it clear that a fixed `TypeId` is returned and not the `TypeId` of the implementor.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use std::any::{Any, TypeId};
    ///
    /// let any_box: Box<dyn Any> = Box::new(42_i32);
    /// assert_eq!(any_box.type_id(), TypeId::of::<i32>()); // ⚠️ this fails!
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::any::{Any, TypeId};
    ///
    /// let any_box: Box<dyn Any> = Box::new(42_i32);
    /// assert_eq!((*any_box).type_id(), TypeId::of::<i32>());
    /// //          ^ dereference first, to call `type_id` on `dyn Any`
    /// ```
    #[clippy::version = "1.73.0"]
    pub TYPE_ID_ON_BOX,
    suspicious,
    "calling `.type_id()` on a boxed trait object"
}

/// Checks if the given type is `dyn Any`, or a trait object that has `Any` as a supertrait.
/// Only in those cases will its vtable have a `type_id` method that returns the implementor's
/// `TypeId`, and only in those cases can we give a proper suggestion to dereference the box.
///
/// If this returns false, then `.type_id()` likely (this may have FNs) will not be what the user
/// expects in any case and dereferencing it won't help either. It will likely require some
/// other changes, but it is still worth emitting a lint.
/// See <https://github.com/rust-lang/rust-clippy/pull/11350#discussion_r1544863005> for more details.
fn is_subtrait_of_any(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    if let ty::Dynamic(preds, ..) = ty.kind() {
        preds.iter().any(|p| match p.skip_binder() {
            ExistentialPredicate::Trait(tr) => {
                cx.tcx.is_diagnostic_item(sym::Any, tr.def_id)
                    || cx
                        .tcx
                        .explicit_super_predicates_of(tr.def_id)
                        .iter_identity_copied()
                        .any(|(clause, _)| {
                            matches!(clause.kind().skip_binder(), ty::ClauseKind::Trait(super_tr)
                            if cx.tcx.is_diagnostic_item(sym::Any, super_tr.def_id()))
                        })
            },
            _ => false,
        })
    } else {
        false
    }
}

pub(super) fn check(cx: &LateContext<'_>, receiver: &Expr<'_>, call_span: Span) {
    let recv_adjusts = cx.typeck_results().expr_adjustments(receiver);

    if let Some(Adjustment { target: recv_ty, .. }) = recv_adjusts.last()
        && let ty::Ref(_, ty, _) = recv_ty.kind()
        && let ty::Adt(adt, args) = ty.kind()
        && adt.is_box()
        && let inner_box_ty = args.type_at(0)
        && let ty::Dynamic(..) = inner_box_ty.kind()
    {
        let ty_name = with_forced_trimmed_paths!(ty.to_string());

        span_lint_and_then(
            cx,
            TYPE_ID_ON_BOX,
            call_span,
            format!("calling `.type_id()` on `{ty_name}`"),
            |diag| {
                let derefs = recv_adjusts
                    .iter()
                    .filter(|adj| matches!(adj.kind, Adjust::Deref(None)))
                    .count();

                diag.note(
                    "this returns the type id of the literal type `Box<_>` instead of the \
                    type id of the boxed value, which is most likely not what you want",
                )
                .note(format!(
                    "if this is intentional, use `TypeId::of::<{ty_name}>()` instead, \
                    which makes it more clear"
                ));

                if is_subtrait_of_any(cx, inner_box_ty) {
                    let mut sugg = "*".repeat(derefs + 1);
                    sugg += &snippet(cx, receiver.span, "<expr>");

                    diag.span_suggestion(
                        receiver.span,
                        "consider dereferencing first",
                        format!("({sugg})"),
                        Applicability::MaybeIncorrect,
                    );
                }
            },
        );
    }
}
