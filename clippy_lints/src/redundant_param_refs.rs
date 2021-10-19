use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_trait_impl_item, match_any_def_paths, paths};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for references to a type parameter with trait bounds that would
    /// already be satisfied by such a reference
    ///
    /// ### Why is this bad?
    ///
    /// It introduces an extra reference if the caller has a type that satisfies
    /// the trait bounds
    ///
    /// ### Example
    /// ```rust
    /// use std::io::{self, Read};
    ///
    /// fn before<R: Read>(reader: &mut R) {}
    /// fn after<R: Read>(reader: R) {}
    ///
    /// let mut stdin = io::stdin();
    /// before(&mut stdin);
    ///
    /// // Still works with a reference due to `Read`
    /// // having `impl<R: Read + ?Sized> Read for &mut R`
    /// after(&mut stdin);
    /// // But no longer requires one
    /// after(stdin);
    /// ```
    pub REDUNDANT_PARAM_REFS,
    nursery,
    "Checks for references to type parameters that can be removed"
}

declare_lint_pass!(RedundantParamRefs => [REDUNDANT_PARAM_REFS]);

fn matches_trait(cx: &LateContext<'_>, mutability: Mutability, trait_id: DefId) -> bool {
    let muts = &[
        &paths::FMT_DEBUG[..],
        &paths::FMT_DISPLAY,
        &paths::FMT_WRITE,
        &paths::IO_BUFREAD,
        &paths::IO_READ,
        &paths::IO_SEEK,
        &paths::IO_WRITE,
    ];

    let nonmuts = &[&paths::FMT_DEBUG[..], &paths::FMT_DISPLAY];

    match mutability {
        Mutability::Mut => match_any_def_paths(cx, trait_id, muts).is_some(),
        Mutability::Not => match_any_def_paths(cx, trait_id, nonmuts).is_some(),
    }
}

fn check_bounds(cx: &LateContext<'_>, bounds: &[&GenericBound<'_>], mutability: Mutability) -> bool {
    if bounds.is_empty() {
        return false;
    }

    bounds.iter().all(|bound| {
        if_chain! {
            if let Some(trait_ref) = bound.trait_ref();
            if let Res::Def(DefKind::Trait, trait_id) = trait_ref.path.res;
            then {
                matches_trait(cx, mutability, trait_id)
            }
            else {
                false
            }
        }
    })
}

fn applicable_bounds<'tcx>(
    cx: &LateContext<'tcx>,
    generic_param: &GenericParam<'tcx>,
    target: DefId,
) -> Option<Vec<&'tcx GenericBound<'tcx>>> {
    let mut bounds: Vec<&GenericBound<'_>> = generic_param.bounds.iter().collect();

    for predicate in cx.generics?.where_clause.predicates {
        if_chain! {
            if let WherePredicate::BoundPredicate(bound_pred) = predicate;
            if let Some(node) = cx.tcx.hir().find(bound_pred.bounded_ty.hir_id);
            if let Node::Ty(ty) = node;
            if let TyKind::Path(q_path) = &ty.kind;
            if let QPath::Resolved(_, path) = q_path;
            if path.res.opt_def_id() == Some(target);
            then {
                bounds.extend(bound_pred.bounds.iter());
            }
        };
    }

    Some(bounds)
}

fn check_decl(cx: &LateContext<'_>, decl: &FnDecl<'_>) {
    for input in decl.inputs {
        if_chain! {
            if let TyKind::Rptr(_, ref_mut_ty) = &input.kind;
            if let TyKind::Path(q_path) = &ref_mut_ty.ty.kind;
            if let QPath::Resolved(_, path) = q_path;
            if let Res::Def(DefKind::TyParam, ty_param_id) = path.res;
            if let Some(node) =  cx.tcx.hir().get_if_local(ty_param_id);
            if let Node::GenericParam(generic_param) = node;

            if let Some(bounds) = applicable_bounds(cx, generic_param, ty_param_id);
            if check_bounds(cx, &bounds, ref_mut_ty.mutbl);
            then {
                span_lint_and_then(
                    cx,
                    REDUNDANT_PARAM_REFS,
                    input.span,
                    "Redundant reference to type parameter",
                    |diag| {
                        diag.span_suggestion_verbose(
                            input.span.until(ref_mut_ty.ty.span),
                            "Remove this reference",
                            String::new(),
                            Applicability::MaybeIncorrect,
                        );
                    }
                );
            }
        }
    }
}

impl LateLintPass<'_> for RedundantParamRefs {
    fn check_fn(
        &mut self,
        cx: &LateContext<'_>,
        _: FnKind<'_>,
        decl: &FnDecl<'_>,
        _: &Body<'_>,
        _: Span,
        hir_id: HirId,
    ) {
        if is_trait_impl_item(cx, hir_id) {
            return;
        }

        check_decl(cx, decl);
    }

    fn check_trait_item(&mut self, cx: &LateContext<'_>, item: &TraitItem<'_>) {
        if let TraitItemKind::Fn(sig, TraitFn::Required(_)) = &item.kind {
            check_decl(cx, sig.decl);
        }
    }
}
