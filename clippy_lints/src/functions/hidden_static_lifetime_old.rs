use crate::rustc_lint::LintContext;
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use rustc_hir::{
    intravisit::{FnKind, Visitor},
    BareFnTy, FnDecl, FnRetTy, GenericArg, GenericArgs, GenericBound, GenericParam, GenericParamKind, Generics,
    Lifetime, LifetimeParamKind, MutTy, ParamName, PolyTraitRef, QPath, Ty, TyKind, TypeBindingKind, WherePredicate,
};
use rustc_lint::LateContext;
use rustc_middle::lint::in_external_macro;
use rustc_span::Span;

use super::HIDDEN_STATIC_LIFETIME;

// As a summary:
//
// A lifetime can only be changed if:
// * Used in immutable references.
// * Not behind a mutable reference.
// * Not used in function types
//
// NOTE: Struct's fields follow the same rules as types

pub(super) fn check_fn<'tcx>(cx: &LateContext<'_>, kind: FnKind<'tcx>, decl: &'tcx FnDecl<'_>, span: Span) {
    if !in_external_macro(cx.sess(), span) &&
            let FnKind::ItemFn(_, generics, _) = kind {

        let mut v = FuncVisitor {
            ret_ty: &decl.output,
            inputs: decl.inputs,
			predicates: &generics.predicates,
            lifetime_is_used: false
        };

        for param in generics.params {
            if let GenericParamKind::Lifetime { kind } = param.kind && kind != LifetimeParamKind::Elided {
                v.visit_generic_param(param);
                if !v.lifetime_is_used {
                    span_lint(cx, HIDDEN_STATIC_LIFETIME, param.span, "hi");
                };
            }
        };
	}
}

struct FuncVisitor<'a> {
    ret_ty: &'a FnRetTy<'a>,
    inputs: &'a [Ty<'a>],
    predicates: &'a [WherePredicate<'a>],
    lifetime_is_used: bool,
}

impl<'v> Visitor<'_> for FuncVisitor<'v> {
    fn visit_generic_param(&mut self, param: &GenericParam<'_>) {
        // Check inputs
		for input in self.inputs {
            if ref_uses_lifetime(input, param) || check_path(input, param) {
				dbg!("@@@@@@@@@@@@@@@@@@@@@@@@");
                self.lifetime_is_used = true;
                return;
            };
        }

        // Check return
        if let FnRetTy::Return(ret_ty) = self.ret_ty {
            if ref_uses_lifetime(ret_ty, param) || check_path(ret_ty, param) {
				dbg!("============================");
                self.lifetime_is_used = true;
                return;
            };
        };

        // Check predicates

        for predicate in self.predicates {
            for bound in predicate.bounds() {
                if let GenericBound::Outlives(lifetime) = bound &&
				lifetime.ident.name == param.name.ident().name {
					self.lifetime_is_used = true;
					return;
                }
            }
        }
    }
}

fn ref_uses_lifetime(mut ty: &Ty<'_>, lifetime: &GenericParam<'_>) -> bool {
    while let TyKind::Ref(lt_ref, mut_ty) = &ty.kind {
		if lt_ref.ident.name == lifetime.name.ident().name && mut_ty.mutbl.is_not() {
			return true;
        } else {
            ty = mut_ty.ty;
        }
    }
    false
}

fn check_path(ty: &Ty<'_>, lifetime: &GenericParam<'_>) -> bool {
    if let TyKind::Path(QPath::Resolved(_, path)) = ty.peel_refs().kind {
        for segment in path.segments {
            for arg in segment.args().args {
                if let GenericArg::Lifetime(lt_arg) = arg {
                    if lt_arg.ident.name == lifetime.name.ident().name {
                        return true;
                    };
                } else if let &GenericArg::Type(ty) = arg {
					dbg!("!!!!!!!!!!!!!!!!!!!!!!!!!");
                    return check_all_types(ty, lifetime);
                };
            }
        }
    };
    false
}

fn check_all_types(ty: &Ty<'_>, lifetime: &GenericParam<'_>) -> bool {
    fn ty_uses_lifetime(ty: &Ty<'_>, generic: &GenericParam<'_>) -> bool {
        if let TyKind::Path(QPath::Resolved(_, path)) = ty.kind {
            for segment in path.segments {
                if let Some(GenericArgs { args, .. }) = segment.args {
                    for arg in args.iter() {
                        if let GenericArg::Lifetime(lifetime) = arg {
							if lifetime.ident.name == generic.name.ident().name {
                                // generic is used
                                return true;
                            }
                        } else if let GenericArg::Type(ty) = arg {
                            return classify_ty(ty, generic);
                        }
                    }
                }
            }
        }
        false
    }

    #[inline]
    fn barefn_uses_lifetime(barefn: &BareFnTy<'_>, generic: &GenericParam<'_>) -> bool {
        // Check inputs
        for input in barefn.decl.inputs {
            if ref_uses_lifetime(input, generic) || check_path(input, generic) {
                return false;
            };
        }

        // Check return
        if let FnRetTy::Return(ret_ty) = barefn.decl.output {
            if check_path(ret_ty, generic) {
                return false;
            };
        };
        true
    }

    #[inline]
    fn tuple_uses_lifetime(tuple: &[Ty<'_>], generic: &GenericParam<'_>) -> bool {
        tuple.iter().any(|ty| classify_ty(ty, generic))
    }

    fn opaquedef_uses_lifetime(args: &[GenericArg<'_>], generic: &GenericParam<'_>) -> bool {
        for arg in args.iter() {
            if let GenericArg::Lifetime(lifetime) = arg {
                if lifetime.ident.name == generic.name.ident().name {
                    // generic is used
                    return true;
                }
            } else if let GenericArg::Type(ty) = arg {
                return classify_ty(ty, generic);
            }
        }
        false
    }

    #[inline]
    fn traitobject_uses_lifetime(lifetime: &Lifetime, traits: &[PolyTraitRef<'_>], generic: &GenericParam<'_>) -> bool {
        if lifetime.ident.name == generic.name.ident().name {
            return true;
        }
        for PolyTraitRef {
            bound_generic_params, ..
        } in traits
        {
            if bound_generic_params.iter().any(|param| param.def_id == generic.def_id) {
                return true;
            };
        }
        false
    }

    #[inline]
    fn classify_ty(ty: &Ty<'_>, generic: &GenericParam<'_>) -> bool {
        match &ty.kind {
            TyKind::Slice(ty) | TyKind::Array(ty, _) => ty_uses_lifetime(ty, generic),
            TyKind::Ptr(mut_ty) => ty_uses_lifetime(mut_ty.ty, generic),
            TyKind::BareFn(barefnty) => barefn_uses_lifetime(barefnty, generic),
            TyKind::Tup(tuple) => tuple_uses_lifetime(tuple, generic),
            TyKind::Path(_) => ty_uses_lifetime(ty, generic),
            TyKind::OpaqueDef(_, genericargs, _) => opaquedef_uses_lifetime(genericargs, generic),
            TyKind::TraitObject(poly_trait_ref, lifetime, _) =>
            	traitobject_uses_lifetime(lifetime, poly_trait_ref, generic),
            TyKind::Typeof(_) // This is unused for now, this needs revising when Typeof is used.
            | TyKind::Err
            | TyKind::Never => false,
			TyKind::Ref(_, MutTy { ty, ..}) => ref_uses_lifetime(ty, generic),
            TyKind::Infer => true,
        }
    }

    // Separate refs from ty.

    // Now final_ty is equivalent to ty.peel_refs
    return classify_ty(ty.peel_refs(), lifetime);
}
