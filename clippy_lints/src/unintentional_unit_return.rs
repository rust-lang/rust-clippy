use crate::utils::{get_trait_def_id, paths, span_lint};
use if_chain::if_chain;
use rustc_middle::ty::{GenericPredicates, Predicate, ProjectionPredicate, TraitPredicate};
use rustc_hir::{Expr, ExprKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Checks for functions that expect closures of type
    /// Fn(...) -> Ord where the implemented closure has a semi-colon
    /// at the end of the last statement.
    ///
    /// **Why is this bad?** Likely the semi-colon is unintentional which
    /// returns () instead of the result of last statement. Since () implements Ord
    /// it doesn't cause a compilation error
    ///
    /// **Known problems:** If returning unit is intentional, then there is no
    /// way of specifying this without triggering needless_return lint
    ///
    /// **Example:**
    ///
    /// ```rust
    /// let mut twins = vec!((1,1), (2,2));
    /// twins.sort_by_key(|x| { x.1; });
    /// ```
    pub UNINTENTIONAL_UNIT_RETURN,
    nursery,
    "fn arguments of type Fn(...) -> Once having last statements with a semi-colon, suggesting to remove the semi-colon if it is unintentional."
}

declare_lint_pass!(UnintentionalUnitReturn => [UNINTENTIONAL_UNIT_RETURN]);

fn unwrap_trait_pred<'tcx>(cx: &LateContext<'_, 'tcx>, pred: &Predicate<'tcx>) -> Option<TraitPredicate<'tcx>> {
    if let Predicate::Trait(poly_trait_pred, _) = pred {
        let trait_pred = cx.tcx.erase_late_bound_regions(&poly_trait_pred);
        Some(trait_pred)
    } else {
        None
    }
}

fn get_predicates_for_trait_path<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    generics: GenericPredicates<'tcx>,
    trait_path: &[&str],
) -> Vec<&'tcx Predicate<'tcx>> {
    let mut preds = Vec::new();
    generics.predicates.iter().for_each(|(pred, _)| {
        if_chain! {
            if let Some(trait_pred) = unwrap_trait_pred(cx, pred);
            if let Some(trait_def_id) = get_trait_def_id(cx, trait_path);
            if trait_def_id == trait_pred.trait_ref.def_id;
            then {
                    preds.push(pred);
                }
        }
    });
    preds
}

fn get_projection_pred<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    generics: GenericPredicates<'tcx>,
    pred: TraitPredicate<'tcx>,
) -> Option<ProjectionPredicate<'tcx>> {
    generics.predicates.iter().find_map(|(proj_pred, _)| {
        if let Predicate::Projection(proj_pred) = proj_pred {
            let projection_pred = cx.tcx.erase_late_bound_regions(proj_pred);
            if projection_pred.projection_ty.substs == pred.trait_ref.substs {
                return Some(projection_pred);
            }
        }
        None
    })
}

fn get_args_to_check<'tcx>(cx: &LateContext<'_, 'tcx>, expr: &'tcx Expr<'tcx>) -> Vec<(usize, String)> {
    let mut args_to_check = Vec::new();
    if let Some(def_id) = cx.tables.type_dependent_def_id(expr.hir_id) {
        let fn_sig = cx.tcx.fn_sig(def_id);
        let generics = cx.tcx.predicates_of(def_id);
        let fn_mut_preds = get_predicates_for_trait_path(cx, generics, &paths::FN_MUT);
        let ord_preds = get_predicates_for_trait_path(cx, generics, &paths::ORD);
        let partial_ord_preds = get_predicates_for_trait_path(cx, generics, &paths::PARTIAL_ORD);
        // Trying to call erase_late_bound_regions on fn_sig.inputs() gives the following error
        // The trait `rustc::ty::TypeFoldable<'_>` is not implemented for `&[&rustc::ty::TyS<'_>]`
        let inputs_output = cx.tcx.erase_late_bound_regions(&fn_sig.inputs_and_output());
        inputs_output.iter().enumerate().for_each(|(i, inp)| {
            // Ignore output param
            if i == inputs_output.len() - 1 {
                return;
            }
            fn_mut_preds.iter().for_each(|pred| {
                if_chain! {
                    if let Some(trait_pred) = unwrap_trait_pred(cx, pred);
                    if trait_pred.self_ty() == *inp;
                    if let Some(return_ty_pred) = get_projection_pred(cx, generics, trait_pred);
                    then {
                        if ord_preds.iter().any(|ord|  {
                            unwrap_trait_pred(cx, ord).unwrap().self_ty()  == return_ty_pred.ty
                        }) {
                            args_to_check.push((i, "Ord".to_string()));
                        }
                        else if partial_ord_preds.iter().any(|pord| {
                            unwrap_trait_pred(cx, pord).unwrap().self_ty()  == return_ty_pred.ty
                        }) {
                            args_to_check.push((i, "PartialOrd".to_string()));
                        }
                    }
                }
            });
        });
    }
    args_to_check
}

fn check_arg<'tcx>(cx: &LateContext<'_, 'tcx>, arg: &'tcx Expr<'tcx>) -> Option<&'tcx Stmt<'tcx>> {
    if let ExprKind::Closure(_, _fn_decl, body_id, _span, _) = arg.kind {
        if_chain! {
            let body = cx.tcx.hir().body(body_id);
            if let ExprKind::Block(block, _) = body.value.kind;
            if let Some(stmt) = block.stmts.last();
            if let StmtKind::Semi(_) = stmt.kind;
            then { return Some(stmt) }
        }
    }
    return None;
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnintentionalUnitReturn {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr<'tcx>) {
        let arg_indices = get_args_to_check(cx, expr);
        if_chain! {
            if let ExprKind::MethodCall(_, _, ref args) = expr.kind;
            then {
                for (i, trait_name) in arg_indices {
                    if_chain! {
                        if i < args.len();
                        if let Some(stmt) = check_arg(cx, &args[i]);
                        then {
                            //TODO : Maybe only filter the closures where the last statement return type also is an unit
                            span_lint(cx,
                                      UNINTENTIONAL_UNIT_RETURN,
                                      stmt.span,
                                      &format!(
                                      "Semi-colon on the last line of this closure returns \
                                       the unit type which also implements {}.",
                                      trait_name));
                        }
                    }
                }
            }
        }
    }
}
