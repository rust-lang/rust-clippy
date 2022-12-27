use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{
    intravisit::FnKind, FnDecl, FnRetTy, GenericArg, GenericBound, GenericParamKind, LifetimeParamKind, QPath, TyKind,
    WherePredicate,
};
use rustc_lint::LateContext;

use super::HIDDEN_STATIC_LIFETIME;

pub(super) fn check_fn<'tcx>(cx: &LateContext<'_>, kind: FnKind<'tcx>, decl: &'tcx FnDecl<'_>) {
    if let FnKind::ItemFn(_, generics, _) = kind {
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
					if let TyKind::Rptr(lifetime, _) = input.kind {
						if lifetime.ident == generic.name.ident() {
							lifetime_is_used = true;
						}
					}
					// If input is struct using a lifetime
					if let TyKind::Path(qpath) = &input.kind {
						if let QPath::Resolved(_, path) = qpath {
							for segment in path.segments {
								for arg in segment.args().args {
									// If input's lifetime and function's are the same.
									if let GenericArg::Lifetime(lifetime) = arg {
										if lifetime.hir_id.owner == generic.hir_id.owner {
											lifetime_is_used = true;
										}
									}
								}
							}
						}
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
										span_lint_and_sugg(cx,
											HIDDEN_STATIC_LIFETIME,
											bound.span(),
											"This lifetime can be changed to `'static`",
											"try",
											"changing this lifetime to 'static".into(),
											Applicability::MachineApplicable
										);
									}
								}
							} else {
								// Check for other lifetimes
								if let WherePredicate::RegionPredicate(region_predicate) = predicate {
									if region_predicate.lifetime.hir_id.owner == generic.hir_id.owner {
										lifetime_is_used = true;
									} else {
										span_lint_and_sugg(cx,
											HIDDEN_STATIC_LIFETIME,
											region_predicate.span,
											"This lifetime can be changed to `'static`",
											"try",
											"changing this lifetime to 'static".into(),
											Applicability::MachineApplicable
										);
									}
								}
							}
						}

						// Check again.
						if !lifetime_is_used {
							span_lint_and_sugg(cx,
								HIDDEN_STATIC_LIFETIME,
								generic.span,
								"This lifetime can be changed to `'static`",
								"try",
								"changing this lifetime to 'static".into(),
								Applicability::MachineApplicable
							);
						}
					}
				}

			}
        }
    }
}
