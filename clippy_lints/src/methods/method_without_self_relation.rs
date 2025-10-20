use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{FnSig, ImplItem, ImplItemKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::METHOD_WITHOUT_SELF_RELATION;

/// Check if a type contains a reference to `Self` anywhere in its structure.
/// This includes direct references and generic parameters.
fn contains_self<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, self_ty: Ty<'tcx>) -> bool {
    // Direct comparison with Self type
    if ty == self_ty {
        return true;
    }

    match ty.kind() {
        // Check if this is a reference to Self
        ty::Ref(_, inner_ty, _) => contains_self(cx, *inner_ty, self_ty),

        // Check if this is a raw pointer to Self
        ty::RawPtr(inner_ty, _) => contains_self(cx, *inner_ty, self_ty),

        // Check generic types like Option<Self>, Vec<Self>, Result<Self, E>, etc.
        ty::Adt(_, args) => args.types().any(|arg_ty| contains_self(cx, arg_ty, self_ty)),

        // Check tuples like (Self, i32) or (String, Self)
        ty::Tuple(types) => types.iter().any(|ty| contains_self(cx, ty, self_ty)),

        // Check array types like [Self; 10]
        ty::Array(elem_ty, _) | ty::Slice(elem_ty) => contains_self(cx, *elem_ty, self_ty),

        // Check function pointer types
        ty::FnPtr(sig, _) => {
            let sig = sig.skip_binder();
            sig.inputs().iter().any(|&ty| contains_self(cx, ty, self_ty)) || contains_self(cx, sig.output(), self_ty)
        },

        // Check closures (uncommon but possible)
        ty::Closure(_, args) => {
            args.as_closure()
                .sig()
                .inputs()
                .skip_binder()
                .iter()
                .any(|&ty| contains_self(cx, ty, self_ty))
                || contains_self(cx, args.as_closure().sig().output().skip_binder(), self_ty)
        },

        // Check opaque types (impl Trait, async fn return types)
        ty::Alias(ty::AliasTyKind::Opaque, alias_ty) => {
            // Check the bounds of the opaque type
            alias_ty.args.types().any(|arg_ty| contains_self(cx, arg_ty, self_ty))
        },

        // Check trait objects (dyn Trait)
        ty::Dynamic(predicates, _) => {
            use rustc_middle::ty::ExistentialPredicate;
            // Check if any of the trait bounds reference Self
            predicates.iter().any(|predicate| match predicate.skip_binder() {
                ExistentialPredicate::Trait(trait_ref) => {
                    trait_ref.args.types().any(|arg_ty| contains_self(cx, arg_ty, self_ty))
                },
                ExistentialPredicate::Projection(projection) => {
                    projection.args.types().any(|arg_ty| contains_self(cx, arg_ty, self_ty))
                        || contains_self(cx, projection.term.as_type().unwrap(), self_ty)
                },
                ExistentialPredicate::AutoTrait(_) => false,
            })
        },

        _ => false,
    }
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'_>, self_ty: Ty<'tcx>) {
    if let ImplItemKind::Fn(ref sig, _) = impl_item.kind {
        // Get the method signature from the type system
        let method_sig = cx.tcx.fn_sig(impl_item.owner_id).instantiate_identity();
        let method_sig = method_sig.skip_binder();

        // Check if there's a self parameter (self, &self, &mut self, self: Arc<Self>, etc.)
        if has_self_parameter(sig) {
            return;
        }

        // Check all input parameters for Self references
        for &param_ty in method_sig.inputs().iter() {
            if contains_self(cx, param_ty, self_ty) {
                return;
            }
        }

        // Check return type for Self references
        let return_ty = method_sig.output();
        if contains_self(cx, return_ty, self_ty) {
            return;
        }

        // If we reach here, the method has no relationship to Self
        emit_lint(cx, impl_item.span, impl_item.ident.name.as_str());
    }
}

/// Check if the function signature has an explicit self parameter
fn has_self_parameter(sig: &FnSig<'_>) -> bool {
    sig.decl.implicit_self.has_implicit_self()
}

fn emit_lint(cx: &LateContext<'_>, span: Span, method_name: &str) {
    span_lint_and_help(
        cx,
        METHOD_WITHOUT_SELF_RELATION,
        span,
        format!("method `{method_name}` has no relationship to `Self`"),
        None,
        "consider making this a standalone function instead of a method",
    );
}
