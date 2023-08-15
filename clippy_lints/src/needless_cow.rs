use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{fn_has_unsatisfiable_preds, is_diag_trait_item};
use itertools::Itertools;
use rustc_const_eval::interpret::{AllocId, AllocRange, ConstValue, GlobalAlloc};
use rustc_data_structures::fx::{FxIndexMap, IndexEntry};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{
    self, AggregateKind, ConstantKind, Location, Operand, Place, Rvalue, SourceInfo, Statement, StatementKind,
    TerminatorKind,
};
use rustc_middle::ty::TyCtxt;
use rustc_mir_dataflow::lattice::FlatSet;
use rustc_mir_dataflow::value_analysis::{Map, State, ValueAnalysis, ValueAnalysisWrapper, ValueOrPlace};
use rustc_mir_dataflow::{Analysis, Results, ResultsVisitor};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Span};
use rustc_target::abi::{FieldIdx, Size};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for locals which are assigned a `Cow`, yet the `Borrowed` variant is only ever used for
    /// empty string slices.
    ///
    /// ### Why is this bad?
    /// An empty string does not allocate any memory. This can use `String` instead, with the
    /// `Borrowed` constructor substituted for `String::new`.
    ///
    /// ### Known problems
    /// If the `Cow` is passed to a function expecting `Cow`, or is mutably borrowed, this will
    /// still lint.
    ///
    /// ### Example
    /// ```rust
    /// # use std::borrow::Cow;
    ///
    /// # let my_owned_string = String::new();
    /// # let something = true;
    /// # let other_thing = true;
    ///
    /// let _ = if something {
    ///     Cow::Owned(my_owned_string);
    /// } else if other_thing {
    ///     Cow::Owned(String::new());
    /// } else {
    ///     Cow::Borrowed("");
    /// };
    /// ```
    /// Use instead:
    /// ```rust
    /// # use std::borrow::Cow;
    ///
    /// # let my_owned_string = String::new();
    /// # let something = true;
    /// # let other_thing = true;
    ///
    /// let _ = if something {
    ///     my_owned_string;
    /// } else if other_thing {
    ///     String::new();
    /// } else {
    ///     String::new();
    /// };
    /// ```
    #[clippy::version = "1.73.0"]
    pub NEEDLESS_COW,
    nursery,
    "usage of `Cow` with only empty strings in `Borrow` variant"
}
impl_lint_pass!(NeedlessCow => [NEEDLESS_COW]);

pub struct NeedlessCow;

impl<'tcx> LateLintPass<'tcx> for NeedlessCow {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        _: &'tcx Body<'tcx>,
        _: Span,
        def_id: LocalDefId,
    ) {
        // Probably unnecessary nowadays considering dummy MIR is created for these functions.
        if fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id);
        let mut cow_locals_to_assignments = FxIndexMap::default();

        for local in mir
            .local_decls
            .iter_enumerated()
            .filter(|(_, decl)| is_type_diagnostic_item(cx, decl.ty, sym::Cow))
            .map(|(i, _)| i)
        {
            cow_locals_to_assignments.insert(local, vec![]);
        }

        let mut results = NeedlessCowAnalysis {
            map: Map::from_filter(cx.tcx, mir, |ty| ty.peel_refs().is_str(), None),
        }
        .wrap()
        .into_engine(cx.tcx, mir)
        .iterate_to_fixpoint();

        results.visit_reachable_with(
            mir,
            &mut NeedlessCowVisitor {
                cx,
                cow_locals_to_assignments: &mut cow_locals_to_assignments,
            },
        );

        for assignments in cow_locals_to_assignments.values() {
            if assignments.len() == 1 {
                continue;
            }
            let Some(first) = assignments.first() else {
                continue;
            };

            span_lint_hir_and_then(
                cx,
                NEEDLESS_COW,
                mir.source_scopes[first.source_info().scope]
                    .local_data
                    .as_ref()
                    .assert_crate_local()
                    .lint_root,
                assignments.iter().map(|a| a.source_info().span).collect_vec(),
                "usage of `Cow` where the `Borrowed` variant is only ever used for empty strings",
                |diag| {
                    diag.help("remove the `Cow` and use `String::new` instead, as it allocates no memory");

                    for into_or_from in assignments.iter().filter(|a| matches!(a, CowAssignment::IntoOrFrom(_))) {
                        diag.span_note(into_or_from.source_info().span, "this implicitly constructs `Borrowed`");
                    }
                },
            );
        }
    }
}

type CowAssignments = Vec<CowAssignment>;

#[derive(Clone, Copy)]
enum CowAssignment {
    Borrowed(SourceInfo),
    Owned(SourceInfo),
    IntoOrFrom(SourceInfo),
}

impl CowAssignment {
    fn source_info(self) -> SourceInfo {
        match self {
            Self::Borrowed(s) | Self::Owned(s) | Self::IntoOrFrom(s) => s,
        }
    }
}

struct NeedlessCowAnalysis {
    map: Map,
}

impl<'tcx> ValueAnalysis<'tcx> for NeedlessCowAnalysis {
    type Value = FlatSet<ConstantKind<'tcx>>;

    const NAME: &'static str = "NeedlessCowAnalysis";

    fn map(&self) -> &Map {
        &self.map
    }

    fn handle_assign(&self, target: Place<'tcx>, rvalue: &Rvalue<'tcx>, state: &mut State<Self::Value>) {
        state.flood(target.as_ref(), &self.map);

        let Some(target_idx) = self.map.find(target.as_ref()) else {
            return;
        };

        let result = match rvalue {
            Rvalue::Use(Operand::Constant(box constant)) => ValueOrPlace::Value(FlatSet::Elem(constant.literal)),
            Rvalue::Use(Operand::Move(place) | Operand::Copy(place))
            | Rvalue::Ref(_, _, place)
            | Rvalue::CopyForDeref(place) => {
                if (place.projection.is_empty() || place.is_indirect_first_projection() && place.projection.len() == 1)
                    // Clear projections if this is either a deref or has no projections. Not sure
                    // why this is needed, but otherwise this is always `Top`. It's probably because
                    // an assignment to `_5` and an access to `*_5` are considered different. This
                    // should be ok since dereferencing a `str` is not possible, and dereferencing a
                    // `&str` has no side effects. (We're filtering by `str`)
                    && let Some(place_idx) = self.map.find(Place::from(place.local).as_ref())
                {
                    ValueOrPlace::Place(place_idx)
                } else {
                    // Not sure why we can't use a guard here, but `place_idx` is apparently
                    // uninitialized
                    self.super_assign(target, rvalue, state);
                    return;
                }
            },
            _ => {
                self.super_assign(target, rvalue, state);
                return;
            },
        };

        state.insert_idx(target_idx, result, &self.map);
    }
}

struct NeedlessCowVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    cow_locals_to_assignments: &'a mut FxIndexMap<mir::Local, CowAssignments>,
}

impl<'mir, 'tcx> ResultsVisitor<'mir, 'tcx, Results<'tcx, ValueAnalysisWrapper<NeedlessCowAnalysis>>>
    for NeedlessCowVisitor<'_, 'tcx>
{
    type FlowState = State<FlatSet<ConstantKind<'tcx>>>;

    fn visit_statement_before_primary_effect(
        &mut self,
        results: &Results<'tcx, ValueAnalysisWrapper<NeedlessCowAnalysis>>,
        state: &Self::FlowState,
        stmt: &'mir Statement<'tcx>,
        _: Location,
    ) {
        let Self {
            cx,
            cow_locals_to_assignments,
        } = self;

        let StatementKind::Assign(box (target, rvalue)) = &stmt.kind else {
            return;
        };

        match rvalue {
            Rvalue::Aggregate(box AggregateKind::Adt(def_id, variant, args, _, None), init)
                if let Some(arg) = init.get(FieldIdx::from(0u32))
                    && let ty = cx.tcx.type_of(def_id).instantiate(cx.tcx, args)
                    && is_type_diagnostic_item(cx, ty, sym::Cow)
                    && let Some(def) = ty.ty_adt_def()
                    && let IndexEntry::Occupied(mut entry) = cow_locals_to_assignments.entry(target.local) =>
            {
                // Only used for its span
                if def.variant(*variant).name == sym!(Owned) {
                    entry.get_mut().push(CowAssignment::Owned(stmt.source_info));
                    return;
                }

                let FlatSet::Elem(val) = (match arg {
                    Operand::Move(arg_place) | Operand::Copy(arg_place) => {
                        state.get(arg_place.as_ref(), &results.analysis.0.map)
                    },
                    Operand::Constant(constant) => FlatSet::Elem(constant.literal),
                }) else {
                    return;
                };
                let val = val.eval(cx.tcx, cx.param_env);

                if is_empty_str(cx.tcx, val) {
                    entry.get_mut().push(CowAssignment::Borrowed(stmt.source_info));
                } else {
                    entry.remove_entry();
                }
            }
            _ => {},
        }
    }

    fn visit_terminator_before_primary_effect(
        &mut self,
        results: &Results<'tcx, ValueAnalysisWrapper<NeedlessCowAnalysis>>,
        state: &Self::FlowState,
        term: &'mir mir::Terminator<'tcx>,
        _: Location,
    ) {
        let Self {
            cx,
            cow_locals_to_assignments,
        } = self;

        // Handle `Into::into` and `From::from` as well
        if let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &term.kind
            && let Some((def_id, _)) = func.const_fn_def()
            && let [arg] = args.as_slice()
            && (is_diag_trait_item(cx, def_id, sym::Into) || is_diag_trait_item(cx, def_id, sym::From))
            && let IndexEntry::Occupied(mut entry) = cow_locals_to_assignments.entry(destination.local)
        {
            let FlatSet::Elem(val) = (match arg {
                Operand::Move(arg_place) | Operand::Copy(arg_place) => {
                    state.get(arg_place.as_ref(), &results.analysis.0.map)
                },
                Operand::Constant(constant) => FlatSet::Elem(constant.literal),
            }) else {
                return;
            };
            let value = val.eval(cx.tcx, cx.param_env);

            if is_empty_str(cx.tcx, value) {
                entry.get_mut().push(CowAssignment::IntoOrFrom(term.source_info));
            } else {
                entry.remove_entry();
            }
        }
    }
}

fn is_empty_str<'tcx>(tcx: TyCtxt<'tcx>, value: ConstantKind<'tcx>) -> bool {
    let ConstantKind::Val(value, ty) = value else {
        return false;
    };
    if !ty.peel_refs().is_str() {
        return false;
    }

    // String slices are represented like this (if they aren't promoted). If either `start` or
    // `end` aren't `0`, this isn't empty.
    if let ConstValue::Slice { start, end, .. } = value {
        return start == 0 && end == 0;
    }

    // A promoted string slice
    if let ConstValue::Scalar(scalar) = value
        && let Ok(p) = scalar.to_pointer(&tcx)
        && let Some(alloc_id) = p.provenance
    {
        return is_alloc_empty_str(tcx, alloc_id);
    }

    false
}

/// This is pretty hacky, but for anything like `&*""` we don't have a simple `const ""`, instead,
/// we have a promoted constant! So we must traverse the evaluated constant backwards until we find
/// the final reference, then we can just check if its bytes are empty. Pretty hacky, but I (and
/// maybe others) have written worse.
fn is_alloc_empty_str(tcx: TyCtxt<'_>, alloc_id: AllocId) -> bool {
    let Some(GlobalAlloc::Memory(alloc)) = tcx.try_get_global_alloc(alloc_id) else {
        return false;
    };

    let alloc = alloc.inner();
    let range = AllocRange {
        start: Size::from_bytes(0),
        size: alloc.size(),
    };

    if !alloc.provenance().range_empty(range, &tcx) {
        let Ok(scalar) = alloc.read_scalar(
            &tcx,
            AllocRange {
                size: tcx.data_layout.pointer_size,
                ..range
            },
            true,
        ) else {
            return false;
        };
        let Ok(ptr) = scalar.to_pointer(&tcx) else {
            return false;
        };
        let Some(alloc_id) = ptr.provenance else {
            return false;
        };

        return is_alloc_empty_str(tcx, alloc_id);
    }

    alloc
        .get_bytes_strip_provenance(&tcx, range)
        .is_ok_and(<[u8]>::is_empty)
}
