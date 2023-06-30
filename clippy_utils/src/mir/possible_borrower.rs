use crate::ty::is_copy;
use rustc_borrowck::consumers::BodyWithBorrowckFacts;
use rustc_index::bit_set::BitSet;
use rustc_lint::LateContext;
use rustc_middle::mir;
use rustc_middle::ty::{self, visit::TypeVisitor, RegionKind, RegionVid, Ty, TyCtxt};
use rustc_mir_dataflow::{impls::MaybeStorageLive, Analysis, ResultsCursor};
use std::borrow::Cow;
use std::ops::ControlFlow;
use std::rc::Rc;

/// Result of `PossibleBorrowerAnalysis`.
#[allow(clippy::module_name_repetitions)]
pub struct PossibleBorrowerMap<'b, 'tcx> {
    body_with_facts: Rc<BodyWithBorrowckFacts<'tcx>>,
    maybe_live: ResultsCursor<'b, 'tcx, MaybeStorageLive<'b>>,
}

impl<'b, 'tcx> PossibleBorrowerMap<'b, 'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, body_with_facts: &'b Rc<BodyWithBorrowckFacts<'tcx>>) -> Self {
        let body = &body_with_facts.body;

        let maybe_live = MaybeStorageLive::new(Cow::Owned(BitSet::new_empty(body.local_decls.len())))
            .into_engine(tcx, body)
            .pass_name("possible_borrower")
            .iterate_to_fixpoint()
            .into_results_cursor(body);

        PossibleBorrowerMap {
            body_with_facts: body_with_facts.clone(),
            maybe_live,
        }
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
        let mir = &self.body_with_facts.body;
        if is_copy(cx, cx.tcx.erase_regions(mir.local_decls[borrowed].ty)) {
            return true;
        }

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
        !self.body_with_facts.borrow_set.local_map[&borrowed]
            .iter()
            .any(|index| {
                let root_vid = self.body_with_facts.borrow_set.location_map[index.as_usize()].region;

                maybe_live.iter().any(|local| {
                    let local_regions = collect_regions(self.body_with_facts.body.local_decls[local].ty);

                    local_regions.iter().any(|&local_vid| {
                        if !self
                            .body_with_facts
                            .region_inference_context
                            .eval_outlives(root_vid, local_vid)
                        {
                            return false;
                        }

                        !borrowers
                            .iter()
                            .filter_map(|&borrower| self.borrower_vid(borrower))
                            .any(|borrower_vid| {
                                self.body_with_facts
                                    .region_inference_context
                                    .eval_outlives(borrower_vid, local_vid)
                            })
                    })
                })
            })
    }

    fn borrower_vid(&self, borrower: mir::Local) -> Option<RegionVid> {
        if let ty::Ref(region, _, _) = self.body_with_facts.body.local_decls[borrower].ty.kind()
            && let ty::RegionKind::ReVar(borrower_vid) = region.kind()
        {
            Some(borrower_vid)
        } else {
            None
        }
    }

    pub fn local_is_alive_at(&mut self, local: mir::Local, at: mir::Location) -> bool {
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

impl<'tcx> TypeVisitor<TyCtxt<'tcx>> for RegionCollector {
    fn visit_region(&mut self, region: ty::Region<'tcx>) -> ControlFlow<Self::BreakTy> {
        if let RegionKind::ReVar(vid) = region.kind() {
            self.0.push(vid);
        }
        ControlFlow::Continue(())
    }
}
