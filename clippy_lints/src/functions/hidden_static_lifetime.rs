use crate::rustc_lint::LintContext;
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use rustc_hir::{
    intravisit::{FnKind, Visitor},
    BareFnTy, FnDecl, FnRetTy, GenericArg, GenericArgs, GenericBound, GenericParam, GenericParamKind, Generics,
    Lifetime, LifetimeParamKind, MutTy, ParamName, PolyTraitRef, QPath, Ty, TyKind, TypeBindingKind, WherePredicate,
};
use rustc_lint::LateContext;
use rustc_middle::lint::in_external_macro;
use rustc_span::{symbol::Ident, Span};

use super::HIDDEN_STATIC_LIFETIME;

struct V<'a> {
	// (Lifetime, Bounded typ)
    lifetimes: Vec<&'a GenericParam<'a>>,
}

impl<'a> Visitor<'_> for V<'a> {
	/// Remove all mutable lifetimes that aren't for T: 'static
    fn visit_where_predicate(&mut self, predicate: &WherePredicate<'_>) {
		if let WherePredicate::BoundPredicate(pred) = predicate {
			for bound in pred.bounds {
				// Check (for each lifetime) that the type they're associated with is 'static.
				continue;
			}
		}
    }

    fn visit_ty(&mut self, ty: &Ty<'_>) {
        let mut outer_continue: bool;
        let mut outer_break = false;
        let mut i = 0;
        while i < self.lifetimes.len() {
            outer_continue = false;
            let lifetime = self.lifetimes[i];

            // Check references

            let mut final_ty = ty;
            let mut behind_mut_ref = false;
            while let TyKind::Ref(lt, mut_ty) = &final_ty.kind {
                if mut_ty.mutbl.is_mut() {
                    behind_mut_ref = true;
                };

                if lt.ident.name == lifetime.name.ident().name || behind_mut_ref {
					if self.lifetimes.is_empty() {
						outer_continue = true;
						continue;
					}

                    self.lifetimes.remove(i);
                    outer_continue = true;
                    
                }
                final_ty = mut_ty.ty;
            }

            if outer_continue {
                continue;
            } else if outer_break {
                break;
            }

            // Now final_ty is equal to ty.peel_refs()
            // Check Paths:

            if let TyKind::Path(QPath::Resolved(_, path)) = final_ty.kind {
                for segment in path.segments {
                    for argument in segment.args().args {
                        if let GenericArg::Lifetime(lt) = argument && lt.ident.name == lifetime.name.ident().name {
							self.lifetimes.remove(i);
							outer_continue = true;
							continue;
						};
                    }
                    if outer_continue {
                        break;
                    }
                }
            };

            if outer_continue {
                continue;
            }
            i += 1;
        }
    }

    fn visit_fn_ret_ty<'v>(&mut self, ret_ty: &'v FnRetTy<'v>) {
        if let FnRetTy::Return(ty) = ret_ty {
            let mut i = 0;
			while i < self.lifetimes.len() {
                dbg!(self.lifetimes[i].name.ident().as_str());
				if ty_uses_lifetime(ty, &self.lifetimes[i], &self) {
                    self.lifetimes.remove(i);
                }
                i += 1;
            }
        }
    }
}

pub(super) fn check_fn<'tcx>(cx: &LateContext<'_>, kind: FnKind<'tcx>, decl: &'tcx FnDecl<'_>, span: Span) {
	if !in_external_macro(cx.sess(), span) &&
	let FnKind::ItemFn(_, generics, _) = kind {
		let mut visitor = V {
			lifetimes: Vec::new()
		};

		// Fill visitor.lifetimes with function's lifetimes

		for generic in generics.params {
			if let GenericParamKind::Lifetime { .. } = generic.kind {
				visitor.lifetimes.push(generic);
			};
		};

		for input in decl.inputs {
			visitor.visit_ty(input);
		}

		for predicate in generics.predicates {
			visitor.visit_where_predicate(predicate);
		}
		visitor.visit_fn_ret_ty(&decl.output);

		for generic in visitor.lifetimes {
			span_lint_and_help(cx,
				HIDDEN_STATIC_LIFETIME,
				generic.span,
				"this lifetime can be changed to `'static`",
				None,
			&format!("try removing the lifetime parameter `{}` and changing references to `'static`", generic.name.ident().as_str()));
		}
	};
}

fn ty_uses_lifetime(ty: &Ty<'_>, generic: &GenericParam, v: &V<'_>) -> bool {
    fn check_ty(ty: &Ty<'_>, generic: &GenericParam, v: &V<'_>) -> bool {
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
                            return classify_ty(ty, generic, v);
                        }
                    }
                }
            }
        }
        false
    }

    #[inline]
    fn barefn_uses_lifetime(barefn: &BareFnTy<'_>, generic: &GenericParam, v: &V<'_>) -> bool {
        // let mut visitor = V {
        //     lifetimes: v.lifetimes.clone(),
        // };

        // for input in barefn.decl.inputs {
        //     visitor.visit_ty(input);
        // }

        // visitor.visit_fn_ret_ty(&barefn.decl.output);
        // !visitor.lifetimes.contains(&generic)
		true
	}

    #[inline]
    fn tuple_uses_lifetime(tuple: &[Ty<'_>], generic: &GenericParam, v: &V<'_>) -> bool {
        tuple.iter().any(|ty| classify_ty(ty, generic, v))
    }

    fn opaquedef_uses_lifetime(args: &[GenericArg<'_>], generic: &GenericParam, v: &V<'_>) -> bool {
        for arg in args.iter() {
            if let GenericArg::Lifetime(lifetime) = arg {
                if lifetime.ident.name == generic.name.ident().name {
                    // generic is used
                    return true;
                }
            } else if let GenericArg::Type(ty) = arg {
                return classify_ty(ty, generic, v);
            }
        }
        false
    }

    #[inline]
    fn traitobject_uses_lifetime(lifetime: &Lifetime, traits: &[PolyTraitRef<'_>], generic: &GenericParam<'_>) -> bool {
        if lifetime.ident.name == generic.name.ident().name {
            return false;
        }
        for PolyTraitRef {
            bound_generic_params, ..
        } in traits
        {
            if bound_generic_params
                .iter()
                .any(|param| param.name.ident().name == generic.name.ident().name)
            {
                return true;
            };
        }
        false
    }

    #[inline]
    fn classify_ty(ty: &Ty<'_>, generic: &GenericParam, v: &V<'_>) -> bool {
        match &ty.kind {
            TyKind::Slice(ty) | TyKind::Array(ty, _) => check_ty(ty, generic, v),
            TyKind::Ptr(mut_ty) => check_ty(mut_ty.ty, generic, v),
            TyKind::BareFn(barefnty) => {
				barefn_uses_lifetime(barefnty, generic, v)},
            TyKind::Tup(tuple) => tuple_uses_lifetime(tuple, generic, v),
            TyKind::Path(_) => check_ty(ty, generic, v),
            TyKind::OpaqueDef(_, genericargs, _) => opaquedef_uses_lifetime(genericargs, generic, v),
            TyKind::TraitObject(poly_trait_ref, lifetime, _) =>
            	traitobject_uses_lifetime(lifetime, poly_trait_ref, generic),
            TyKind::Typeof(_) // This is unused for now, this needs revising when Typeof is used.
            | TyKind::Err
            | TyKind::Never
			| TyKind::Ref(_, _)
			| TyKind::Infer => true,
        }
    }

    return classify_ty(ty.peel_refs(), &generic, v);
}
