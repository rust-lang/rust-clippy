// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::rustc::hir::Expr;
use crate::rustc::infer::InferCtxt;
use crate::rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use crate::rustc::traits;
use crate::rustc::ty::adjustment::{Adjust, PointerCast};
use crate::rustc::ty::{self, ToPolyTraitRef, Ty};
use crate::rustc::{declare_lint_pass, declare_tool_lint};
use crate::syntax_pos::symbol::Ident;
use crate::utils::{match_def_path, paths, span_lint_and_then};
use if_chain::if_chain;
use std::collections::VecDeque;

declare_clippy_lint! {
    /// **What it does:** Checks for coercing something that already contains a
    /// `dyn Any` to `dyn Any` itself.
    ///
    /// **Why is this bad?** It's probably a mistake.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// # use std::any::Any;
    /// struct Foo;
    /// let box_foo: Box<Foo> = Box::new(Foo);
    /// let mut box_any: Box<dyn Any> = box_foo;
    /// let bad: &mut dyn Any = &mut box_any;
    /// // you probably meant
    /// let ok: &mut dyn Any = &mut *box_any;
    /// ```
    pub WRONG_ANY_COERCE,
    correctness,
    "coercing a type already containing `dyn Any` to `dyn Any` itself"
}

declare_lint_pass!(WrongAnyCoerce => [WRONG_ANY_COERCE]);

struct LintData<'tcx> {
    coerced_to_any: Ty<'tcx>,
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for WrongAnyCoerce {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        let adjustments = cx.tables.expr_adjustments(expr);
        for (i, adj) in adjustments.iter().enumerate() {
            if let Adjust::Pointer(PointerCast::Unsize) = adj.kind {
                let src_ty = if i == 0 {
                    cx.tables.expr_ty(expr)
                } else {
                    adjustments[i - 1].target
                };
                cx.tcx.infer_ctxt().enter(|infcx| {
                    let opt_lint_data = check_unsize_coercion(cx, &infcx, cx.param_env, src_ty, adj.target);
                    if let Some(lint_data) = opt_lint_data {
                        // TODO: we might be able to suggest dereferencing in some cases
                        let cta_str = lint_data.coerced_to_any.to_string();
                        span_lint_and_then(
                            cx,
                            WRONG_ANY_COERCE,
                            expr.span,
                            &format!("coercing `{}` to `dyn Any`", cta_str),
                            |db| {
                                if !cta_str.contains("Any") {
                                    db.note(&format!("`{}` dereferences to `dyn Any`", cta_str));
                                }
                            },
                        )
                    }
                });
            }
        }
    }
}

/// Returns whether or not this coercion should be linted
fn check_unsize_coercion<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    infcx: &InferCtxt<'_, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    src_ty: Ty<'tcx>,
    tgt_ty: Ty<'tcx>,
) -> Option<LintData<'tcx>> {
    // redo the typechecking for this coercion to see if it required unsizing something to `dyn Any`
    // see https://github.com/rust-lang/rust/blob/cae6efc37d70ab7d353e6ab9ce229d59a65ed643/src/librustc_typeck/check/coercion.rs#L454-L611
    let tcx = infcx.tcx;
    let coerce_unsized_trait_did = tcx.lang_items().coerce_unsized_trait().unwrap();
    let unsize_trait_did = tcx.lang_items().unsize_trait().unwrap();

    // don't report overflow errors
    let mut selcx = traits::SelectionContext::with_query_mode(&infcx, traits::TraitQueryMode::Canonical);
    let mut queue = VecDeque::new();
    queue.push_back(
        ty::TraitRef::new(coerce_unsized_trait_did, tcx.mk_substs_trait(src_ty, &[tgt_ty.into()])).to_poly_trait_ref(),
    );
    while let Some(trait_ref) = queue.pop_front() {
        if_chain! {
            if trait_ref.def_id() == unsize_trait_did;
            if is_type_dyn_any(cx, trait_ref.skip_binder().input_types().nth(1).unwrap());
            // found something unsizing to `dyn Any`
            let coerced_to_any = trait_ref.self_ty();
            if type_contains_any(cx, &mut selcx, param_env, coerced_to_any);
            then {
                return Some(LintData { coerced_to_any });
            }
        }
        let select_result = selcx.select(&traits::Obligation::new(
            traits::ObligationCause::dummy(),
            param_env,
            trait_ref.to_poly_trait_predicate(),
        ));
        if let Ok(Some(vtable)) = select_result {
            // we only care about trait predicates for these traits
            let traits = [coerce_unsized_trait_did, unsize_trait_did];
            queue.extend(
                vtable
                    .nested_obligations()
                    .into_iter()
                    .filter_map(|oblig| oblig.predicate.to_opt_poly_trait_ref())
                    .filter(|tr| traits.contains(&tr.def_id())),
            );
        }
    }
    None
}

fn type_contains_any<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    selcx: &mut traits::SelectionContext<'_, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    // check if it derefs to `dyn Any`
    if_chain! {
        if let Some((any_src_deref_ty, _deref_count)) = fully_deref_type(selcx, param_env, ty);
        if is_type_dyn_any(cx, any_src_deref_ty);
        then {
            // TODO: use deref_count to make a suggestion
            return true;
        }
    }
    // TODO: check for `RefCell<dyn Any>`?
    false
}

fn is_type_dyn_any<'tcx>(cx: &LateContext<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    if_chain! {
        if let ty::Dynamic(trait_list, _) = ty.sty;
        if let Some(principal_trait) = trait_list.skip_binder().principal();
        if match_def_path(cx, principal_trait.def_id, &paths::ANY_TRAIT);
        then {
            return true;
        }
    }
    false
}

/// Calls [`deref_type`] repeatedly
fn fully_deref_type<'tcx>(
    selcx: &mut traits::SelectionContext<'_, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    src_ty: Ty<'tcx>,
) -> Option<(Ty<'tcx>, usize)> {
    if let Some(deref_1) = deref_type(selcx, param_env, src_ty) {
        let mut deref_count = 1;
        let mut cur_ty = deref_1;
        while let Some(deref_n) = deref_type(selcx, param_env, cur_ty) {
            deref_count += 1;
            cur_ty = deref_n;
        }
        Some((cur_ty, deref_count))
    } else {
        None
    }
}

/// Returns the type of `*expr`, where `expr` has type `src_ty`.
/// This will go through `Deref` `impl`s if necessary.
/// Returns `None` if `*expr` would not typecheck.
fn deref_type<'tcx>(
    selcx: &mut traits::SelectionContext<'_, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    src_ty: Ty<'tcx>,
) -> Option<Ty<'tcx>> {
    if let Some(ty::TypeAndMut { ty, .. }) = src_ty.builtin_deref(true) {
        Some(ty)
    } else {
        // compute `<T as Deref>::Target`
        let infcx = selcx.infcx();
        let tcx = selcx.tcx();
        let src_deref = ty::TraitRef::new(
            tcx.lang_items().deref_trait().unwrap(),
            tcx.mk_substs_trait(src_ty, &[]),
        );
        let mut obligations = Vec::new();
        let src_deref_ty = traits::normalize_projection_type(
            selcx,
            param_env,
            ty::ProjectionTy::from_ref_and_name(tcx, src_deref, Ident::from_str("Target")),
            traits::ObligationCause::dummy(),
            0,
            &mut obligations,
        );
        // only return something if all the obligations definitely hold
        let obligations_ok = obligations
            .iter()
            .all(|oblig| infcx.predicate_must_hold_considering_regions(oblig));
        if obligations_ok {
            Some(infcx.resolve_vars_if_possible(&src_deref_ty))
        } else {
            None
        }
    }
}
