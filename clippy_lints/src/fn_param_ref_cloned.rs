use clippy_utils::res::MaybeResPath;
use clippy_utils::ty::implements_trait;
use clippy_utils::visitors::for_each_expr;
use rustc_hir::{Body, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ref, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks if a function clones a parameter passed by reference.
    ///
    /// ### Why is this bad?
    /// Caller should decide where to copy and place data.
    /// The function should not hide the need of ownership of data.
    ///
    /// ### Example
    /// ```norun
    /// #[derive(Clone)]
    /// struct A;
    ///
    /// pub fn foo(item: &A) {
    ///     let cloned_ref = item.clone();
    /// }
    /// ```
    #[clippy::version = "1.98.0"]
    pub FN_PARAM_REF_CLONED,
    pedantic,
    "you should pass by value instead of cloning a passed reference"
}

impl_lint_pass!(FnParamRefCloned => [FN_PARAM_REF_CLONED]);

type CandidateId = rustc_hir::HirId;
type CandidateSpan = Span;
type Candidate = (CandidateId, CandidateSpan);
type CandidateRebinds = Vec<Candidate>;

#[derive(Default)]
pub struct FnParamRefCloned {
    candidates: Vec<(Candidate, CandidateRebinds)>,
}

pub fn is_candidate_ty<'a>(cx: &LateContext<'a>, ty: Ty<'a>, must_impl_trait: &[DefId]) -> bool {
    if let Ref(_, ty_ref, _) = ty.kind() {
        must_impl_trait
            .iter()
            .any(|def_id| implements_trait(cx, *ty_ref, *def_id, &[]))
    } else {
        false
    }
}

pub fn get_param_id_span(param: &rustc_hir::Param<'_>) -> Option<(rustc_hir::HirId, Span)> {
    if let PatKind::Binding(_, hir_id, ident, _) = param.pat.kind {
        if !ident.span.from_expansion() && !ident.is_reserved() {
            Some((hir_id, param.ty_span))
        } else {
            None
        }
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for FnParamRefCloned {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        fn_body: &'tcx rustc_hir::Body<'tcx>,
        _: Span,
        def_id: rustc_span::def_id::LocalDefId,
    ) {
        // Define which traits must be implemented for the lint to work
        let must_impl_trait = [
            cx.tcx.lang_items().clone_trait().unwrap(),
            cx.tcx.lang_items().drop_trait().unwrap(),
        ];

        // Get all candidates of params that implement said traits and zip them with function signature
        // params
        self.candidates = cx
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_binder()
            .inputs()
            .into_iter()
            .zip(fn_body.params)
            .filter_map(|(ty, param)| {
                if let Some((id, span)) = get_param_id_span(param)
                    && is_candidate_ty(cx, *ty, &must_impl_trait)
                {
                    Some(((id, span), Vec::default()))
                } else {
                    None
                }
            })
            .collect();

        // Find all rebinds of param values in the function and add them to the original candidates (tuple)
        if let rustc_hir::ExprKind::Block(block, _) = fn_body.value.kind {
            for statement in block.stmts.iter() {
                if let rustc_hir::StmtKind::Let(let_stmt) = statement.kind
                    && let Some(expr) = let_stmt.init
                    && let rustc_hir::ExprKind::Path(qpath) = expr.kind
                    && let Some(hir_id) = qpath.res_local_id()
                {
                    self.candidates.iter_mut().for_each(|(cand, relat)| {
                        if cand.0 == hir_id {
                            relat.push((let_stmt.pat.hir_id, let_stmt.span));
                        }
                    });
                }
            }
        }

        // Look whether the candidates call the `.clone()` method anywhere
        _ = for_each_expr::<(), ()>(cx, fn_body.value, move |x| {
            match x.kind {
                rustc_hir::ExprKind::MethodCall(method_name, receiver, args, span)
                    if method_name.ident.as_str() == "clone"
                        && args.is_empty()
                        && let rustc_hir::ExprKind::Path(qpath) = receiver.kind
                        && let Some(hir_id) = qpath.res_local_id() =>
                {
                    self.candidates.iter().for_each(|(original_candidate, rebinds)| {
                        if original_candidate.0 == hir_id {
                            clippy_utils::diagnostics::span_lint_and_note(
                                cx,
                                FN_PARAM_REF_CLONED,
                                span,
                                "function gets a parameter by reference, but you later unconditionally clone it",
                                Some(original_candidate.1),
                                "consider passing the reference by value instead",
                            );
                        }

                        for rebind in rebinds {
                            if rebind.0 == hir_id {
                                clippy_utils::diagnostics::span_lint_and_then(
                                    cx,
                                    FN_PARAM_REF_CLONED,
                                    span,
                                    "function gets a parameter by reference, but you later rebind and unconditionally clone it",
                                    |diag| {
                                        diag
                                            .span_note(rebind.1, "you bind the parameter into a new binding here")
                                            .span_note(original_candidate.1, "the parameter is passed by reference...");
                                    },
                                );
                            }
                        }
                    });
                },
                _ => (),
            };

            ControlFlow::Continue(())
        })
    }
}
