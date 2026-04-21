use std::collections::hash_map::Entry;
use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::mir::{PossibleBorrowerMap, enclosing_mir, expr_local};
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef as _, MaybeResPath};
use clippy_utils::source::snippet;
use clippy_utils::sym;
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_ast::Mutability;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::mir::Local;

use super::MAP_UNWRAP_OR;

/// lint use of `map().unwrap_or_else()` for `Option`s and `Result`s
///
/// Is part of the `map_unwrap_or` lint, split into separate files for readability.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    map_arg: &'tcx hir::Expr<'_>,
    unwrap_arg: &'tcx hir::Expr<'_>,
    msrv: Msrv,
) -> bool {
    let recv_ty = cx.typeck_results().expr_ty(recv).peel_refs();
    let recv_ty_kind = match recv_ty.opt_diag_name(cx) {
        Some(sym::Option) => sym::Option,
        Some(sym::Result) if msrv.meets(cx, msrvs::RESULT_MAP_OR_ELSE) => sym::Result,
        _ => return false,
    };

    // Don't make a suggestion that may fail to compile due to mutably borrowing
    // the same variable twice.
    if let Some(mir) = enclosing_mir(cx.tcx, expr.hir_id) {
        let borrower_map = PossibleBorrowerMap::new(cx, mir);
        let recv_borrowed_vars = borrowed_variables(cx, recv, &borrower_map);
        let map_borrowed_vars = borrowed_variables(cx, map_arg, &borrower_map);
        let unwrap_borrowed_vars = borrowed_variables(cx, unwrap_arg, &borrower_map);
        if borrows_in_conflict(&recv_borrowed_vars, &map_borrowed_vars)
            || borrows_in_conflict(&recv_borrowed_vars, &unwrap_borrowed_vars)
            || borrows_in_conflict(&map_borrowed_vars, &unwrap_borrowed_vars)
        {
            return false;
        }
    }

    // lint message
    let msg = if recv_ty_kind == sym::Option {
        "called `map(<f>).unwrap_or_else(<g>)` on an `Option` value"
    } else {
        "called `map(<f>).unwrap_or_else(<g>)` on a `Result` value"
    };
    // get snippets for args to map() and unwrap_or_else()
    let map_snippet = snippet(cx, map_arg.span, "..");
    let unwrap_snippet = snippet(cx, unwrap_arg.span, "..");
    // lint, with note if both map() and unwrap_or_else() have the same span
    if map_arg.span.eq_ctxt(unwrap_arg.span) {
        let var_snippet = snippet(cx, recv.span, "..");
        span_lint_and_sugg(
            cx,
            MAP_UNWRAP_OR,
            expr.span,
            msg,
            "try",
            format!("{var_snippet}.map_or_else({unwrap_snippet}, {map_snippet})"),
            Applicability::MachineApplicable,
        );
        return true;
    }

    false
}

fn borrowed_variables<'a, 'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
    borrower_map: &'a PossibleBorrowerMap<'a, 'tcx>,
) -> FxHashMap<Local, Mutability> {
    let mut used_vars = FxHashMap::default();
    for_each_expr_without_closures(expr, |expr| {
        if let Some(hir_id) = expr
            .res_local_id()
            // Closures become `Local`s referencing their bodies in the MIR
            .or_else(|| matches!(expr.kind, ExprKind::Closure(_)).then_some(expr.hir_id))
            && let entry = used_vars.entry(hir_id)
            && matches!(entry, Entry::Vacant(_))
            && let Some(local) = expr_local(cx.tcx, expr)
        {
            entry.insert_entry(local);
        }

        ControlFlow::<()>::Continue(())
    });

    let mut borrowed_vars = FxHashMap::default();
    #[expect(
        rustc::potential_query_instability,
        reason = "collecting borrowed variables is not sensitive to ordering"
    )]
    for local in used_vars.values() {
        for (borrower, mutability) in borrower_map
            .map
            .iter()
            .filter_map(|(borrower, borrowed)| borrowed.get(local).map(|mutability| (*borrower, *mutability)))
        {
            match borrowed_vars.entry(borrower) {
                Entry::Vacant(entry) => {
                    entry.insert(mutability);
                },
                Entry::Occupied(mut entry) => {
                    if mutability.is_mut() && entry.get().is_not() {
                        entry.insert(Mutability::Mut);
                    }
                },
            }
        }
    }

    borrowed_vars
}

fn borrows_in_conflict(
    borrowed_vars1: &FxHashMap<Local, Mutability>,
    borrowed_vars2: &FxHashMap<Local, Mutability>,
) -> bool {
    #[expect(
        rustc::potential_query_instability,
        reason = "checking borrowed variables is not sensitive to ordering"
    )]
    borrowed_vars1.iter().any(|(borrower, mutability)| {
        matches!(borrowed_vars2.get(borrower), 
            Some(other_mutability) if mutability.is_mut() || other_mutability.is_mut())
    })
}
