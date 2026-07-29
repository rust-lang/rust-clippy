use clippy_utils::diagnostics::span_lint;
use rustc_abi::ExternAbi;
use rustc_hir::intravisit::{InferKind, Visitor, VisitorExt as _, walk_ty};
use rustc_hir::{self as hir, AmbigArg, GenericParamKind, TyKind};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::TYPE_COMPLEXITY;

pub(super) fn check(cx: &LateContext<'_>, ty: &hir::Ty<'_>, type_complexity_threshold: u64) -> bool {
    let score = type_complexity_score(ty, cx.tcx.features().enabled(sym::type_alias_impl_trait));

    if score > type_complexity_threshold {
        span_lint(
            cx,
            TYPE_COMPLEXITY,
            ty.span,
            "very complex type used. Consider factoring parts into `type` definitions",
        );
        true
    } else {
        false
    }
}

fn type_complexity_score(ty: &hir::Ty<'_>, type_alias_impl_trait_enabled: bool) -> u64 {
    let mut visitor = TypeComplexityVisitor::new(type_alias_impl_trait_enabled);
    visitor.visit_ty_unambig(ty);
    visitor.into_score()
}

fn ambig_type_complexity_score(ty: &hir::Ty<'_, AmbigArg>, type_alias_impl_trait_enabled: bool) -> u64 {
    let mut visitor = TypeComplexityVisitor::new(type_alias_impl_trait_enabled);
    visitor.visit_ty(ty);
    visitor.into_score()
}

/// Walks a type and assigns a complexity score to it.
struct TypeComplexityVisitor {
    /// total complexity score of the type
    score: u64,
    /// highest complexity score found in stable opaque bounds that can be factored out separately
    max_opaque_bound_score: u64,
    /// current nesting level
    nest: u64,
    /// whether opaque `impl Trait` types can be named through `type_alias_impl_trait`
    type_alias_impl_trait_enabled: bool,
}

impl TypeComplexityVisitor {
    fn new(type_alias_impl_trait_enabled: bool) -> Self {
        Self {
            score: 0,
            max_opaque_bound_score: 0,
            nest: 1,
            type_alias_impl_trait_enabled,
        }
    }

    fn into_score(self) -> u64 {
        self.score.max(self.max_opaque_bound_score)
    }
}

impl<'tcx> Visitor<'tcx> for TypeComplexityVisitor {
    fn visit_infer(&mut self, inf_id: hir::HirId, _inf_span: Span, _kind: InferKind<'tcx>) -> Self::Result {
        self.score += 1;
        self.visit_id(inf_id);
    }

    fn visit_ty(&mut self, ty: &'tcx hir::Ty<'_, AmbigArg>) {
        let (add_score, sub_nest) = match ty.kind {
            // &x and *x have only small overhead; don't mess with nesting level
            TyKind::Ptr(..) | TyKind::Ref(..) => (1, 0),

            // the "normal" components of a type: named types, arrays/tuples
            TyKind::Path(..) | TyKind::Slice(..) | TyKind::Tup(..) | TyKind::Array(..) => (10 * self.nest, 1),

            // function types bring a lot of overhead
            TyKind::FnPtr(fn_ptr) if fn_ptr.abi == ExternAbi::Rust => (50 * self.nest, 1),

            TyKind::TraitObject(param_bounds, _) => {
                let has_lifetime_parameters = param_bounds.iter().any(|bound| {
                    bound
                        .bound_generic_params
                        .iter()
                        .any(|param| matches!(param.kind, GenericParamKind::Lifetime { .. }))
                });
                if has_lifetime_parameters {
                    // complex trait bounds like A<'a, 'b>
                    (50 * self.nest, 1)
                } else {
                    // simple trait bounds like A + B
                    (20 * self.nest, 0)
                }
            },

            _ => (0, 0),
        };
        self.score += add_score;
        self.nest += sub_nest;
        if let TyKind::OpaqueDef(opaque_ty) = ty.kind
            && !self.type_alias_impl_trait_enabled
        {
            let mut visitor = OpaqueBoundComplexityVisitor {
                max_score: 0,
                type_alias_impl_trait_enabled: self.type_alias_impl_trait_enabled,
            };
            for bound in opaque_ty.bounds {
                visitor.visit_param_bound(bound);
            }
            self.max_opaque_bound_score = self.max_opaque_bound_score.max(visitor.max_score);
        } else {
            walk_ty(self, ty);
        }
        self.nest -= sub_nest;
    }
}

struct OpaqueBoundComplexityVisitor {
    max_score: u64,
    type_alias_impl_trait_enabled: bool,
}

impl<'tcx> Visitor<'tcx> for OpaqueBoundComplexityVisitor {
    fn visit_ty(&mut self, ty: &'tcx hir::Ty<'_, AmbigArg>) {
        self.max_score = self
            .max_score
            .max(ambig_type_complexity_score(ty, self.type_alias_impl_trait_enabled));
        walk_ty(self, ty);
    }
}
