use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_copy;
use clippy_utils::visitors::for_each_expr;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Ident, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for closure creating raw pointer from owned type and returning raw pointer
    ///
    /// ### Why is this bad?
    /// It might create a dangling pointer becaused owned type in a closure are dropped after the call
    ///
    /// ### Example
    /// ```no_run
    /// let v_opt = Some(vec![1]);
    /// let _v_ptr = v_opt.map(|v| v.as_ptr());
    /// ```
    /// Use instead:
    /// ```no_run
    /// let v_opt = Some(vec![1]);
    /// let _v_ptr = v_opt.as_ref().map(|v| v.as_ptr());
    /// ```
    #[clippy::version = "1.89.0"]
    pub AS_PTR_IN_MAP,
    nursery,
    "check raw pointer inside closure from owned type"
}
declare_lint_pass!(AsPtrInMap => [AS_PTR_IN_MAP]);

impl<'tcx> LateLintPass<'tcx> for AsPtrInMap {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Closure(hir::Closure { body, .. }) = expr.kind {
            let typeck = cx.typeck_results();
            let closure_output = if let ty::Closure(_, closure_subs) = typeck.expr_ty(expr).kind() {
                let closure = closure_subs.as_closure();
                let closure_sig = cx
                    .tcx
                    .signature_unclosure(closure.sig(), hir::Safety::Safe)
                    .skip_binder();

                closure_sig.output()
            } else {
                return;
            };

            // check if the return type of the closure contains a raw pointer
            if ty_contains_raw_ptr(closure_output) {
                let closure_body = cx.tcx.hir_body(*body);

                let mut idents_needs_drop: Vec<_> = closure_body
                    .params
                    .iter()
                    .flat_map(|param| get_ident_bindings(*param.pat))
                    .filter(|(_, hir_id)| {
                        let ty = typeck.node_type(*hir_id);
                        !is_copy(cx, ty) && ty.needs_drop(cx.tcx, cx.typing_env())
                    })
                    .map(|(i, _)| (i, false))
                    .collect();

                // get usage of call that create raw pointer inside the closure
                let _ = for_each_expr(cx, expr, |e| {
                    if let Some(receiver) = is_creating_raw_ptr(*e) {
                        for (ident, is_creating_raw_ptr) in &mut idents_needs_drop {
                            if ident_eq(*ident, receiver) {
                                *is_creating_raw_ptr = true;
                            }
                        }
                    }
                    if false {
                        return ControlFlow::Break(());
                    }
                    ControlFlow::Continue(())
                });

                let indents_make_raw_pointer = idents_needs_drop
                    .iter()
                    .filter_map(|(ident, is_creating_raw_ptr)| is_creating_raw_ptr.then_some(ident))
                    .collect::<Vec<_>>();

                if !indents_make_raw_pointer.is_empty() {
                    span_lint_and_then(
                        cx,
                        AS_PTR_IN_MAP,
                        expr.span,
                        "this closure might return a dangling pointer",
                        |diag| {
                            for ident in indents_make_raw_pointer {
                                diag.span_note(
                                    ident.span,
                                    "this bindings is used to create a raw pointer that might be dangling",
                                );
                            }
                        },
                    );
                }
            }
        }
    }
}

fn ident_eq(name: Ident, path: hir::Expr<'_>) -> bool {
    match path.kind {
        hir::ExprKind::Path(hir::QPath::Resolved(None, path)) => {
            path.segments.len() == 1 && path.segments[0].ident == name
        },
        hir::ExprKind::AddrOf(_, _, expr) => ident_eq(name, *expr),
        _ => false,
    }
}

fn ty_contains_raw_ptr(ty: ty::Ty<'_>) -> bool {
    match ty.kind() {
        #[allow(clippy::match_same_arms)]
        ty::Adt(_, _) => false, // TODO: might contain raw pointer
        ty::Array(ty, _) | ty::Slice(ty) => ty_contains_raw_ptr(*ty),
        ty::RawPtr(_, _) => true,
        ty::Tuple(ty_list) => ty_list.iter().any(|ty| ty_contains_raw_ptr(ty)),
        _ => false,
    }
}

fn get_ident_bindings(pat: hir::Pat<'_>) -> Vec<(Ident, hir::HirId)> {
    match pat.kind {
        hir::PatKind::Binding(hir::BindingMode::NONE | hir::BindingMode::MUT, hir_id, ident, _) => {
            vec![(ident, hir_id)]
        },
        hir::PatKind::Struct(_, pat_fields, _) => pat_fields
            .iter()
            .flat_map(|pat_field| get_ident_bindings(*pat_field.pat))
            .collect(),
        hir::PatKind::TupleStruct(_, pats, _) => pats.iter().flat_map(|pat| get_ident_bindings(*pat)).collect(),
        hir::PatKind::Tuple(pats, _) => pats.iter().flat_map(|pat| get_ident_bindings(*pat)).collect(),
        hir::PatKind::Box(pat) => get_ident_bindings(*pat),
        hir::PatKind::Slice(pats1, pat_opt, pats2) => pats1
            .iter()
            .flat_map(|pat| get_ident_bindings(*pat))
            .chain(pat_opt.iter().flat_map(|pat| get_ident_bindings(**pat)))
            .chain(pats2.iter().flat_map(|pat| get_ident_bindings(*pat)))
            .collect(),
        _ => vec![],
    }
    // });
}

fn is_creating_raw_ptr(expr: hir::Expr<'_>) -> Option<hir::Expr<'_>> {
    match expr.kind {
        hir::ExprKind::MethodCall(method, receiver, [], _) => {
            if is_dangerous_ptr(method.ident.name) {
                return Some(*receiver);
            }
        },
        hir::ExprKind::Call(function, [arg]) => {
            let hir::ExprKind::Path(hir::QPath::TypeRelative(_, path_segment)) = function.kind else {
                return None;
            };

            if is_dangerous_ptr(path_segment.ident.name) {
                return Some(*arg);
            }
        },
        _ => (),
    }

    None
}

fn is_dangerous_ptr(s: Symbol) -> bool {
    matches!(s.as_str(), "as_ptr" | "as_non_null" | "as_mut_ptr")
}
