use crate::rustc_lint::LintContext;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{
    intravisit::FnKind, FnDecl, FnRetTy, GenericArg, GenericBound, GenericParamKind, LifetimeParamKind, ParamName,
    QPath, Ty, TyKind, TypeBindingKind, WherePredicate,
};
use rustc_lint::LateContext;
use rustc_middle::lint::in_external_macro;
use rustc_span::Span;

use super::HIDDEN_STATIC_LIFETIME;

pub(super) fn check_fn<'tcx>(cx: &LateContext<'_>, kind: FnKind<'tcx>, decl: &'tcx FnDecl<'_>, span: Span) {
    if !in_external_macro(cx.sess(), span) && let FnKind::ItemFn(_, generics, _) = kind {
        let mut lifetime_is_used;
        for generic in generics.params.iter() {
            if let GenericParamKind::Lifetime { kind } = generic.kind &&
			kind != LifetimeParamKind::Elided {
				lifetime_is_used = false;
				// Check that inputs don't use this lifetime
				for input in decl.inputs {
					if lifetime_is_used {
						break;
					}
					// If input is reference
					if let TyKind::Rptr(lifetime, mut_ty) = &input.kind {
						if !lifetime.is_anonymous() && lifetime.ident == generic.name.ident() {
								lifetime_is_used = true;
						} else {
							lifetime_is_used = check_if_uses_lifetime(mut_ty.ty, &generic.name);
						}
					} else {
						lifetime_is_used = check_if_uses_lifetime(input, &generic.name);
					}
				}

				if !lifetime_is_used {
					// Check that lifetime is used in return type.
					if let FnRetTy::Return(_) = decl.output {
						// Check that it isn't used in `where` predicate
						for predicate in generics.predicates {
							// Check for generic types: `where A: 'a`
							if let WherePredicate::BoundPredicate(bound_predicate) = predicate {
								for bound in bound_predicate.bounds {
									if let GenericBound::Outlives(_) = bound {
										lifetime_is_used = true;
									} else {
										// Check that generic isn't X<A = B>.
										if let GenericBound::Trait(poly_trait_ref, _) = bound {
											for segment in poly_trait_ref.trait_ref.path.segments {
												if let Some(gen_args) = segment.args {
													for ty_binding in gen_args.bindings {
														if let TypeBindingKind::Equality { .. } = ty_binding.kind {
															lifetime_is_used = true;
														}
													}
												}
											}
										} else {
											span_lint_and_help(cx,
												HIDDEN_STATIC_LIFETIME,
												bound.span(),
												"this lifetime can be changed to `'static`",
												None,
								&format!("try removing the lifetime parameter `{}` and changing references to `'static`", generic.name.ident().as_str()),
											);
										}
									}
								}
							} else {
								// Check for other lifetimes
								if let WherePredicate::RegionPredicate(region_predicate) = predicate {
									if region_predicate.lifetime.hir_id.owner == generic.hir_id.owner {
										lifetime_is_used = true;
									} else {
										span_lint_and_help(cx,
											HIDDEN_STATIC_LIFETIME,
											region_predicate.span,
											"this lifetime can be changed to `'static`",
											None,
								&format!("try removing the lifetime parameter `{}` and changing references to `'static`", generic.name.ident().as_str()),
										);
									}
								}
							}
						}

						// Check again.
						if !lifetime_is_used {
							span_lint_and_help(cx,
								HIDDEN_STATIC_LIFETIME,
								generic.span,
								"this lifetime can be changed to `'static`",
								None,
								&format!("try removing the lifetime parameter `{}` and changing references to `'static`", generic.name.ident().as_str()),
							);
						}
					}
				}

			}
        }
    }
}

fn check_if_uses_lifetime(input: &Ty<'_>, generic_name: &ParamName) -> bool {
    if let TyKind::Path(QPath::Resolved(_, path)) = &input.kind {
        for segment in path.segments {
            for arg in segment.args().args {
                // If input's lifetime and function's are the same.
                if let GenericArg::Lifetime(lifetime) = arg {
                    if lifetime.is_anonymous() {
                        return true;
                    }
                    if let ParamName::Plain(ident) = generic_name {
                        if lifetime.ident.name == ident.name {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
