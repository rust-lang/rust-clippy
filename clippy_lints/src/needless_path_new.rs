use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeTypeckRes as _};
use clippy_utils::sugg::Sugg;
use clippy_utils::{expr_or_init, sym};
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_infer::infer::InferCtxt;
use rustc_infer::traits::{Obligation, ObligationCause};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::{self, ClauseKind, ClausePolarity, GenericClauses, ParamTy, Ty};
use rustc_trait_selection::infer::TyCtxtInferExt as _;
use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt as _;
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    /// Detects expressions being enclosed in `Path::new` when passed to a function which would
    /// accept the enclosed expression directly.
    ///
    /// ### Why is this bad?
    /// It is unnecessarily verbose
    ///
    /// ### Example
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write(Path::new("foo.txt"), "foo");
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write("foo.txt", "foo");
    /// ```
    #[clippy::version = "1.100.0"]
    pub NEEDLESS_PATH_NEW,
    pedantic,
    "`Path::new(x)` passed as an argument where `x` would suffice"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        let tcx = cx.tcx;

        let (fn_did, args) = match e.kind {
            ExprKind::Call(callee, args)
                if let Res::Def(DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(_, CtorKind::Fn), did) =
                    // re: `expr_or_init`: `callee` might be a variable storing a fn ptr, for example,
                    // so we need to get to the actual initializer
                    expr_or_init(cx, callee).res(cx) =>
            {
                (did, args)
            },
            ExprKind::MethodCall(_, _, args, _) if let Some(did) = cx.ty_based_def(e).opt_def_id() => (did, args),
            _ => return,
        };

        let sig = tcx.fn_sig(fn_did).skip_binder().skip_binder();

        let infcx = cx.tcx.infer_ctxt().build(cx.typing_mode());

        // `ExprKind::MethodCall` doesn't include the receiver in `args`, but does in `sig.inputs()`
        // -- so we iterate over both in `rev`erse in order to line them up starting from the _end_
        //
        // and for `ExprKind::Call` this is basically a no-op
        iter::zip(sig.inputs().iter().rev(), args.iter().rev())
            .enumerate()
            .for_each(|(arg_idx, (arg_ty, arg))| {
                // we want `arg` to be `Path::new(x)`
                if let ExprKind::Call(path_new, [x]) = arg.kind
                    && let ExprKind::Path(QPath::TypeRelative(path, new)) = path_new.kind
                    && path.res(cx).is_diag_item(cx, sym::Path)
                    && new.ident.name == sym::new
                    && let ty::Param(arg_param_ty) = arg_ty.kind()
                    && !is_used_anywhere_else(
                        *arg_param_ty,
                        sig.inputs()
                            .iter()
                            // `arg_idx` is based on the reversed order, so we need to reverse as well for the
                            // `enumerate` indices to work
                            .rev()
                            .enumerate()
                            .filter_map(|(i, input)| (i != arg_idx).then_some(*input)),
                    )
                    && let x_ty = cx.typeck_results().expr_ty(x)
                    && has_required_clauses(cx, &infcx, *arg_ty, x_ty, cx.tcx.clauses_of(fn_did))
                {
                    let mut applicability = Applicability::MachineApplicable;
                    let sugg = Sugg::hir_with_applicability(cx, x, "_", &mut applicability);
                    span_lint_and_sugg(
                        cx,
                        NEEDLESS_PATH_NEW,
                        arg.span,
                        "the expression enclosed in `Path::new` can be passed directly",
                        "try",
                        sugg.to_string(),
                        applicability,
                    );
                }
            });
    }
}

fn is_used_anywhere_else<'tcx>(param_ty: ParamTy, mut other_sig_tys: impl Iterator<Item = Ty<'tcx>>) -> bool {
    other_sig_tys.any(|sig_ty| {
        sig_ty.walk().any(|generic_arg| {
            if let Some(ty) = generic_arg.as_type()
                && let ty::Param(pt) = ty.kind()
                && *pt == param_ty
            {
                true
            } else {
                false
            }
        })
    })
}

fn has_required_clauses<'tcx>(
    cx: &LateContext<'tcx>,
    infcx: &InferCtxt<'tcx>,
    param_ty: Ty<'tcx>,
    x_ty: Ty<'tcx>,
    clauses: GenericClauses<'tcx>,
) -> bool {
    let mut has_clauses = false;

    let has_required_clauses = clauses
        .clauses
        .iter()
        .filter_map(|(clause, _)| {
            let clause = clause.kind();
            #[expect(clippy::match_same_arms, reason = "branches have different reasons to be `None`")]
            match clause.skip_binder() {
                // This is what we analyze
                // NOTE: repeats the contents of `Clause::as_trait_clause`,
                // except we don't `Binder::rebind` as we don't care about the binder
                ClauseKind::Trait(trait_clause) => Some(trait_clause),

                // Trivially holds for `P`: `Path::new` has signature `&S -> &Path`, so any "outlives" that holds for
                // `P` does so for `S` as well
                ClauseKind::TypeOutlives(_) | ClauseKind::RegionOutlives(_) => None,

                // Irrelevant to us: neither `AsRef` nor `Sized` have associated types
                ClauseKind::Projection(_) => None,

                // Irrelevant: we don't have anything to do with consts
                ClauseKind::ConstArgHasType(..) | ClauseKind::ConstEvaluatable(_) | ClauseKind::HostEffect(_) => None,

                // Irrelevant?: we don't deal with unstable impls
                ClauseKind::UnstableFeature(_) => None,

                ClauseKind::WellFormed(_) => None,
            }
        })
        .filter(|clause| {
            // dbg!(clause.self_ty(), param_ty);
            clause.self_ty() == param_ty
        })
        .all(|clause| {
            has_clauses = true;

            if clause.polarity != ClausePolarity::Positive {
                return false;
            }

            let new_clause = clause.with_replaced_self_ty(cx.tcx, x_ty);
            let obligation = Obligation::new(cx.tcx, ObligationCause::dummy(), cx.param_env, new_clause);
            infcx.predicate_must_hold_modulo_regions(&obligation)
            //     match cx.tcx.get_diagnostic_name(pred.def_id()) {
            //     Some(sym::AsRef) => {
            //         // TODO: check if it's `AsRef<Path>` in paricular
            //     },
            //     Some(sym::Sized) => todo!(),
            //     _ => return false,
            // };
        });

    if !has_clauses {
        // There were no trait clauses -- this means that the type just needs to be `Path`, so the
        // lint is not applicable
        return false;
    }

    has_required_clauses
}
