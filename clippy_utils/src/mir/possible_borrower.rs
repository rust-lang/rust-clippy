use super::{translate_local, translate_location};
use crate::ty::is_copy;
use rustc_borrowck::{borrow_set::BorrowSet, nll, region_infer::RegionInferenceContext, NllCtxt};
use rustc_hir::def_id::LocalDefId;
use rustc_index::{bit_set::BitSet, vec::IndexVec};
use rustc_infer::infer::{DefiningAnchor, TyCtxtInferExt};
use rustc_lint::LateContext;
use rustc_middle::mir;
use rustc_middle::ty::{self, visit::TypeVisitor, RegionKind, RegionVid, Ty, TyCtxt, WithOptConstParam};
use rustc_mir_build::{build::mir_build_fn_or_const, thir::cx::thir_build};
use rustc_mir_dataflow::{impls::MaybeStorageLive, Analysis, ResultsCursor};
use rustc_mir_transform::{mir_compute_const_qualifs, mir_promote, mir_ready_for_const_eval};
use std::borrow::Cow;
use std::ops::ControlFlow;
use std::rc::Rc;

/// Result of `PossibleBorrowerAnalysis`.
#[allow(clippy::module_name_repetitions)]
pub struct PossibleBorrowerMap<'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    local_def_id: LocalDefId,
    regioncx: Rc<RegionInferenceContext<'tcx>>,
    body: Rc<mir::Body<'tcx>>,
    borrow_set: Rc<BorrowSet<'tcx>>,
    maybe_live: ResultsCursor<'b, 'tcx, MaybeStorageLive<'b>>,
}

impl<'b, 'tcx> PossibleBorrowerMap<'b, 'tcx> {
    pub fn new(cx: &LateContext<'tcx>, local_def_id: LocalDefId) -> Self {
        let tcx = cx.tcx;

        let infcx = tcx
            .infer_ctxt()
            .with_opaque_type_inference(DefiningAnchor::Bind(local_def_id))
            .build();

        let (input_body, input_promoted) = Self::mir_build(tcx, local_def_id);

        let mut nll_ctxt = NllCtxt::new(&infcx, &input_body, &input_promoted);

        let nll::NllOutput { regioncx, .. } = nll_ctxt.compute_regions(false);

        let NllCtxt { body, borrow_set, .. } = nll_ctxt;

        let regioncx = Rc::new(regioncx);
        let body = Rc::new(body);
        let borrow_set = Rc::new(borrow_set);

        let maybe_live = MaybeStorageLive::new(Cow::Owned(BitSet::new_empty(body.local_decls.len())))
            .into_engine(tcx, &body)
            .pass_name("possible_borrower")
            .iterate_to_fixpoint()
            .into_results_cursor(body.clone());

        PossibleBorrowerMap {
            tcx,
            local_def_id,
            regioncx,
            body,
            borrow_set,
            maybe_live,
        }
    }

    fn mir_build(
        tcx: TyCtxt<'tcx>,
        local_def_id: LocalDefId,
    ) -> (mir::Body<'tcx>, IndexVec<mir::Promoted, mir::Body<'tcx>>) {
        let def = WithOptConstParam {
            did: local_def_id,
            const_param_did: None,
        };

        let (thir, expr) = thir_build(tcx, def).expect("`thir_build` should succeed");

        let body_init = mir_build_fn_or_const(tcx, def, thir, expr);

        let body_const = mir_ready_for_const_eval(tcx, body_init);

        let const_qualifs = mir_compute_const_qualifs(tcx, def, || &body_const);

        mir_promote(tcx, const_qualifs, body_const)
    }

    /// Returns true if the set of borrowers of `borrowed` living at `at` includes no more than
    /// `borrowers`.
    /// Notes:
    /// 1. It would be nice if `PossibleBorrowerMap` could store `cx` so that `at_most_borrowers`
    /// would not require it to be passed in. But a `PossibleBorrowerMap` is stored in `LintPass`
    /// `Dereferencing`, which outlives any `LateContext`.
    /// 2. In all current uses of `at_most_borrowers`, `borrowers` is a slice of at most two
    /// elements. Thus, `borrowers.contains(...)` is effectively a constant-time operation. If
    /// `at_most_borrowers`'s uses were to expand beyond this, its implementation might have to be
    /// adjusted.
    pub fn at_most_borrowers(
        &mut self,
        cx: &LateContext<'tcx>,
        borrowers: &[mir::Local],
        borrowed: mir::Local,
        at: mir::Location,
    ) -> bool {
        let mir = cx.tcx.optimized_mir(self.local_def_id.to_def_id());
        if is_copy(cx, mir.local_decls[borrowed].ty) {
            return true;
        }

        let Some(borrowers) = borrowers
            .iter()
            .map(|&borrower| translate_local(cx.tcx, mir, &self.body, borrower))
            .collect::<Option<Vec<_>>>()
         else {
            debug_assert!(false, "can't find {borrowers:?}");
            return false;
        };

        let Some(borrowed) = translate_local(cx.tcx, mir, &self.body, borrowed) else {
            debug_assert!(false, "can't find {borrowed:?}");
            return false;
        };

        let Some(at) = translate_location(cx.tcx, mir, &self.body, at) else {
            debug_assert!(false, "can't find {at:?}: {:?}", mir.stmt_at(at));
            return false;
        };

        self.maybe_live.seek_before_primary_effect(at);

        let maybe_live = self.maybe_live.get();

        // For each borrow of `borrowed`, ask the following question:
        //
        // - Is there any local, live at location `at`, and with an associated region that is outlived by
        //   the borrow region, but that is not also outlived by any of the regions of `borrowers`?
        //
        // If the answer to any of these question is "yes," then there are potential additional borrowers of
        // `borrowed`.
        //
        // Note that the `any` closure has no side effects. So the result is the same regardless of the
        // order in which `index`es are visited.
        #[allow(rustc::potential_query_instability)]
        !self.borrow_set.local_map[&borrowed].iter().any(|index| {
            let root_vid = self.borrow_set.location_map[index.as_usize()].region;

            maybe_live.iter().any(|local| {
                let local_regions = collect_regions(self.body.local_decls[local].ty);

                local_regions.iter().any(|&local_vid| {
                    if !self.regioncx.eval_outlives(root_vid, local_vid) {
                        return false;
                    }

                    !borrowers
                        .iter()
                        .filter_map(|&borrower| self.borrower_vid(borrower))
                        .any(|borrower_vid| self.regioncx.eval_outlives(borrower_vid, local_vid))
                })
            })
        })
    }

    fn borrower_vid(&self, borrower: mir::Local) -> Option<RegionVid> {
        if let ty::Ref(region, _, _) = self.body.local_decls[borrower].ty.kind()
            && let ty::RegionKind::ReVar(borrower_vid) = region.kind()
        {
            Some(borrower_vid)
        } else {
            None
        }
    }

    pub fn local_is_alive_at(&mut self, local: mir::Local, at: mir::Location) -> bool {
        let mir = self.tcx.optimized_mir(self.local_def_id.to_def_id());

        let Some(at) = translate_location(self.tcx, mir, &self.body, at) else {
            debug_assert!(false, "can't find {at:?}: {:?}", mir.stmt_at(at));
            return false;
        };

        self.maybe_live.seek_after_primary_effect(at);
        self.maybe_live.contains(local)
    }
}

fn collect_regions(ty: Ty<'_>) -> Vec<RegionVid> {
    let mut rc = RegionCollector(Vec::new());
    rc.visit_ty(ty);
    rc.0
}

struct RegionCollector(Vec<RegionVid>);

impl<'tcx> TypeVisitor<'tcx> for RegionCollector {
    fn visit_region(&mut self, region: ty::Region<'tcx>) -> ControlFlow<Self::BreakTy> {
        if let RegionKind::ReVar(vid) = region.kind() {
            self.0.push(vid);
        }
        ControlFlow::Continue(())
    }
}
