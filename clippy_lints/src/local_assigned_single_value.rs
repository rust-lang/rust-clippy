use clippy_utils::diagnostics::span_lint;
use clippy_utils::fn_has_unsatisfiable_preds;
use clippy_utils::source::snippet_opt;
use itertools::Itertools;
use rustc_const_eval::interpret::Scalar;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{intravisit::FnKind, Body, FnDecl};
use rustc_index::IndexVec;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{
    self, interpret::ConstValue, visit::Visitor, Constant, Location, Mutability, Operand, Place, Rvalue,
};
use rustc_middle::mir::{AggregateKind, CastKind, PlaceElem, Terminator, TerminatorKind};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::source_map::Spanned;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for locals that are always assigned the same value.
    ///
    /// ### Why is this bad?
    /// It's almost always a typo. If not, it can be made immutable, or turned into a constant.
    ///
    /// ### Example
    /// ```rust
    /// let mut x = 1;
    /// x = 1;
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 1;
    /// ```
    #[clippy::version = "1.72.0"]
    pub LOCAL_ASSIGNED_SINGLE_VALUE,
    correctness,
    "disallows assigning locals many times with the same value"
}
declare_lint_pass!(LocalAssignedSingleValue => [LOCAL_ASSIGNED_SINGLE_VALUE]);

impl LateLintPass<'_> for LocalAssignedSingleValue {
    fn check_fn(
        &mut self,
        cx: &LateContext<'_>,
        _: FnKind<'_>,
        _: &FnDecl<'_>,
        _: &Body<'_>,
        _: Span,
        def_id: LocalDefId,
    ) {
        // Building MIR for `fn`s with unsatisfiable preds results in ICE.
        if fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id.to_def_id());
        let mut v = V {
            body: mir,
            cx,
            map: mir.local_decls.iter().map(|_| LocalUsageValues::default()).collect(),
        };
        v.visit_body(mir);

        for (local, i) in v.map.iter_enumerated() {
            let LocalUsageValues {
                usage,
                mut_ref_acquired,
            } = i;

            if !mut_ref_acquired && eval_nested_locals_are_constant(&v.map, i)
                && eval_local(&v.map, i)
                && let Some(local_decl) = mir.local_decls.get(local)
                    && let [dbg_info] = &*mir
                        .var_debug_info
                        .iter()
                        .filter(|info| info.source_info.span == local_decl.source_info.span)
                        .collect_vec()
                // Don't handle function arguments.
                && dbg_info.argument_index.is_none()
                // Ignore anything from a procedural macro, or locals we cannot prove aren't
                // temporaries
                && let Some(snippet) = snippet_opt(cx, dbg_info.source_info.span)
                && snippet.ends_with(dbg_info.name.as_str())
            {
                span_lint(
                    cx,
                    LOCAL_ASSIGNED_SINGLE_VALUE,
                    usage.iter().map(|i| i.span).collect_vec(),
                    "local only ever assigned single value",
                );
            }
        }
    }
}

type LocalUsageMap<'tcx> = IndexVec<mir::Local, LocalUsageValues<'tcx>>;

/// Holds the data we have for the usage of a local.
#[derive(Debug, Default)]
struct LocalUsageValues<'tcx> {
    /// Where and what this local is assigned.
    usage: Vec<Spanned<LocalUsage<'tcx>>>,
    /// Whether it's mutably borrowed, ever. We should not lint this.
    mut_ref_acquired: bool,
}

#[derive(Debug)]
enum LocalUsage<'tcx> {
    /// A single `Scalar`, for stuff like `i32` or `bool`.
    Scalar(Scalar),
    /// Any number of `Scalar`s. This is used for handling arrays and tuples
    Scalars(Vec<Scalar>),
    /// When a `Local` depends on the value of another local
    DependsOn(mir::Local),
    /// When a `Local` depends on the value of another local by accessing any of its fields or
    /// indexing
    DependsOnWithProj(mir::Local, &'tcx PlaceElem<'tcx>),
    /// Used when a local's assigned a value we cannot prove is constant.
    NonConst,
    /// This is always overwritten.
    Pending,
}

struct V<'a, 'tcx> {
    body: &'a mir::Body<'tcx>,
    cx: &'a LateContext<'tcx>,
    map: LocalUsageMap<'tcx>,
}

impl<'a, 'tcx> Visitor<'tcx> for V<'a, 'tcx> {
    fn visit_assign(&mut self, place: &Place<'tcx>, rvalue: &Rvalue<'tcx>, loc: Location) {
        let Self { body, cx, map } = self;
        let Some(stmt) = body.stmt_at(loc).left() else {
            return;
        };

        if stmt.source_info.span.from_expansion() {
            return;
        }

        // Do not lint if there are any mutable borrows to a local
        if let Rvalue::Ref(_, mir::BorrowKind::Unique | mir::BorrowKind::Mut { .. }, place)
        | Rvalue::AddressOf(Mutability::Mut, place) = rvalue
        {
            map[place.local].mut_ref_acquired = true;
            return;
        }

        let usage = &mut map[place.local];
        let mut spanned = Spanned {
            node: LocalUsage::Pending,
            span: stmt.source_info.span,
        };

        if let Rvalue::Use(operand) = rvalue {
            if let Operand::Constant(constant) = operand
                && let Constant { literal, .. } = **constant
                && let Some(ConstValue::Scalar(scalar)) = literal.try_to_value(cx.tcx)
            {
                spanned.node = LocalUsage::Scalar(scalar);
            } else if let Some(place) = operand.place() {
                if let [base_proj, ..] = place.projection.as_slice()
                    // Handle `let [x, y] = [1, 1]` and `let (x, y) = (1, 1)`
                    && matches!(base_proj, PlaceElem::Field(..) | PlaceElem::Index(..))
                {
                    spanned.node = LocalUsage::DependsOnWithProj(place.local, base_proj);
                } else {
                    // It seems sometimes a local's just moved, no projections, so let's make sure we
                    // don't set `assigned_non_const` to true for these cases
                    spanned.node = LocalUsage::DependsOn(place.local);
                }
            }
        }
        // Handle creation of tuples/arrays, otherwise the above would be useless
        else if let Rvalue::Aggregate(kind, fields) = rvalue
            // TODO: Handle `Adt`s, if possible.
            && let AggregateKind::Array(..) | AggregateKind::Tuple = **kind
            // TODO: Let's remove this `cloned` call, if possible.
            && let Some(scalars) = extract_scalars(cx, fields.into_iter().cloned())
        {
            spanned.node = LocalUsage::Scalars(scalars);
        } else if let Rvalue::Cast(kind, operand, _) = rvalue {
            if let Operand::Constant(constant) = operand
                && matches!(
                    kind,
                    CastKind::IntToInt | CastKind::FloatToInt | CastKind::FloatToFloat | CastKind::IntToFloat,
                )
                && let Constant { literal, .. } = **constant
                && let Some(ConstValue::Scalar(scalar)) = literal.try_to_value(cx.tcx)
            {
                spanned.node = LocalUsage::Scalar(scalar);
            } else if let Operand::Move(place) = operand {
                if let [base_proj, ..] = place.projection.as_slice()
                    && matches!(base_proj, PlaceElem::Field(..) | PlaceElem::Index(..))
                {
                    // Probably unnecessary
                    spanned.node = LocalUsage::DependsOnWithProj(place.local, base_proj);
                } else {
                    // Handle casts from enum discriminants
                    spanned.node = LocalUsage::DependsOn(place.local);
                }
            }
        } else {
            spanned.node = LocalUsage::NonConst;
        }

        if !matches!(spanned.node, LocalUsage::Pending) {
            usage.usage.push(spanned);
        }
    }

    fn visit_terminator(&mut self, terminator: &Terminator<'_>, _: Location) {
        let Self { body: _, cx: _, map } = self;

        if let TerminatorKind::Call { destination, .. } = terminator.kind {
            map[destination.local].usage.push(Spanned {
                node: LocalUsage::NonConst,
                span: terminator.source_info.span,
            });
        }
    }
}

/// `None` means any one of the `Operand`s is not an `Operand::Constant`.
fn extract_scalars<'tcx, O>(cx: &LateContext<'tcx>, operands: O) -> Option<Vec<Scalar>>
where
    O: IntoIterator<Item = Operand<'tcx>>,
{
    operands
        .into_iter()
        .map(|operand| -> Option<_> {
            if let Operand::Constant(constant) = operand
            && let Constant { literal, .. } = *constant
            && let ConstValue::Scalar(scalar) = literal.try_to_value(cx.tcx)?
        {
            return Some(scalar);
        }

            None
        })
        .collect::<Option<Vec<_>>>()
}

fn eval_local(map: &LocalUsageMap<'_>, local: &LocalUsageValues<'_>) -> bool {
    let mut assignments = vec![];

    if local.usage.len() == 1 {
        return false;
    }

    for assignment in &local.usage {
        match assignment.node {
            LocalUsage::Scalar(scalar) => assignments.push(scalar),
            LocalUsage::DependsOn(local) => {
                let [assignment] = &*map[local].usage else {
                    return false;
                };
                if let LocalUsage::Scalar(scalar) = assignment.node {
                    assignments.push(scalar);
                } else {
                    return false;
                }
            },
            LocalUsage::DependsOnWithProj(local, base_proj) => {
                let [assignment] = &*map[local].usage else {
                    return false;
                };
                match base_proj {
                    PlaceElem::Field(idx, _) if let LocalUsage::Scalars(scalars) = &assignment.node => {
                        assignments.push(scalars[idx.as_usize()]);
                    },
                    PlaceElem::Index(idx) if let LocalUsage::Scalars(scalars) = &assignment.node => {
                        assignments.push(scalars[idx.as_usize()]);
                    },
                    _ => return false,
                }
            },
            _ => return false,
        }
    }

    if let Some(assignments) = assignments.iter().map(|i| {
            if let Scalar::Int(int) = i {
                return Some(int.to_bits(int.size()).ok()).flatten();
            };

            None
        })
        .collect::<Option<Vec<_>>>()
        && let [head, tail @ ..] = &*assignments
        && tail.iter().all(|&i| i == *head)
    {
        return true;
    }

    false
}

fn eval_nested_locals_are_constant(map: &LocalUsageMap<'_>, local: &LocalUsageValues<'_>) -> bool {
    eval_nested_locals_are_constant_with_visited_locals(map, local, &mut FxHashSet::default())
}

/// Do not call this manually - use `eval_nested_locals_are_constant` instead.
fn eval_nested_locals_are_constant_with_visited_locals(
    map: &LocalUsageMap<'_>,
    local: &LocalUsageValues<'_>,
    visited_locals: &mut FxHashSet<mir::Local>,
) -> bool {
    let mut constness = true;
    for assignment in &local.usage {
        match assignment.node {
            LocalUsage::DependsOn(local) | LocalUsage::DependsOnWithProj(local, _) => {
                if !visited_locals.insert(local) {
                    // Short-circuit to ensure we don't get stuck in a loop
                    return false;
                }

                constness &= eval_nested_locals_are_constant_with_visited_locals(map, &map[local], visited_locals);
            },
            LocalUsage::NonConst => return false,
            _ => {},
        }
    }
    constness
}
