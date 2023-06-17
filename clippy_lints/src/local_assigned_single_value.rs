use clippy_utils::diagnostics::span_lint;
use clippy_utils::fn_has_unsatisfiable_preds;
use clippy_utils::source::snippet_opt;
use itertools::Itertools;
use rustc_const_eval::interpret::Scalar;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{intravisit::FnKind, Body, FnDecl};
use rustc_index::IndexVec;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{
    self, interpret::ConstValue, visit::Visitor, Constant, Location, Mutability, Operand, Place, Rvalue,
};
use rustc_middle::mir::{AggregateKind, PlaceElem, Terminator, TerminatorKind};
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

        // for (local, i) in v.map.iter_enumerated() {
        //     dbg!(local, i);
        // }

        for (local, i) in v.map.iter_enumerated() {
            if !i.assigned_non_const_rvalue
                && !i.mut_ref_acquired
                && nested_locals_are_not_from_bad_instr(&v.map, i)
                && assignments_all_same_value(&v.map, i)
            {
                let LocalUsageValues {
                    usage,
                    mut_ref_acquired: _,
                    assigned_non_const_rvalue: _,
                    is_from_bad_instr: _,
                } = i;

                if let Some(local_decl) = mir.local_decls.get(local)
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

        /*
        for (local, usage) in &v.local_usage {
            if should_lint(&v.local_usage, *local, usage) {
                let LocalUsageValues {
                    usage,
                    mut_ref_acquired: _,
                    assigned_non_const: _,
                } = usage;

                if let Some(local_decl) = mir.local_decls.get(*local)
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
                        usage.iter().map(|(span, _)| *span).collect_vec(),
                        "local only ever assigned single value",
                    );
                }
            }
        }
        */
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
    /// Whether it's assigned a value we cannot prove is constant, ever. We should not lint this.
    assigned_non_const_rvalue: bool,
    /// Whether it's assigned a value that we know cannot be constant. This is differentiated from
    /// `assigned_non_const` since we check this for nested locals.
    ///
    /// This is set to `true` for function calls or binary operations.
    is_from_bad_instr: bool,
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

        if let Rvalue::Use(Operand::Constant(constant)) = rvalue
            && let Constant { literal, .. } = **constant
            && let Some(ConstValue::Scalar(scalar)) = literal.try_to_value(cx.tcx)
        {
            usage.usage.push(Spanned { node: LocalUsage::Scalar(scalar), span: stmt.source_info.span });
        } else if let Rvalue::Use(operand) = rvalue
            && let Some(place) = operand.place()
        {
            if let [base_proj, ..] = place.projection.as_slice()
                // Handle `let [x, y] = [1, 1]` and `let (x, y) = (1, 1)`
                && matches!(base_proj, PlaceElem::Field(..) | PlaceElem::Index(..))
            {
                usage.usage.push(Spanned {
                    node: LocalUsage::DependsOnWithProj(place.local, base_proj),
                    span: stmt.source_info.span,
                });
            } else {
                // It seems sometimes a local's just moved, no projections, so let's make sure we
                // don't set `assigned_non_const` to true for these cases
                usage.usage.push(Spanned {
                    node: LocalUsage::DependsOn(place.local),
                    span: stmt.source_info.span
                });
            }
        }
        // Handle creation of tuples/arrays, otherwise the above would be useless
        else if let Rvalue::Aggregate(kind, fields) = rvalue
            // TODO: Handle `Adt`s, if possible.
            && let AggregateKind::Array(..) | AggregateKind::Tuple = **kind
            // TODO: Let's remove this `cloned` call, if possible.
            && let Some(scalars) = extract_scalars(cx, fields.into_iter().cloned())
        {
            usage.usage.push(Spanned {
                node: LocalUsage::Scalars(scalars),
                span: stmt.source_info.span,
            })
        } else if let Rvalue::BinaryOp(..) | Rvalue::CheckedBinaryOp(..) = rvalue {
            usage.is_from_bad_instr = true;
        } else {
            // We can also probably handle stuff like `x += 1` here, maybe. But this would be
            // very very complex. Let's keep it simple enough.
            usage.assigned_non_const_rvalue = true;
        }
    }

    fn visit_terminator(&mut self, terminator: &Terminator<'_>, _: Location) {
        let Self { body: _, cx: _, map } = self;

        // TODO: Maybe we can allow const fns, if we can evaluate them of course
        if let TerminatorKind::Call { destination, .. } = terminator.kind {
            map[destination.local].is_from_bad_instr = true;
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

fn assignments_all_same_value(map: &LocalUsageMap<'_>, usage: &LocalUsageValues<'_>) -> bool {
    let LocalUsageValues {
        usage,
        mut_ref_acquired: _,
        assigned_non_const_rvalue: _,
        is_from_bad_instr: _,
    } = usage;

    if usage.len() <= 1 {
        return false;
    }

    // TODO: This code is clone-hell.

    let mut all_assignments = vec![];
    for assignment in usage {
        match &assignment.node {
            LocalUsage::Scalar(scalar) => {
                all_assignments.push(scalar.clone());
            },
            // FIXME: This doesn't handle assignment of tuples, arrays or anything else currently.
            LocalUsage::Scalars(_) => {},
            // FIXME: This doesn't allow nested dependencies, currently.
            // FIXME: This only allows one assignment for dependencies.
            LocalUsage::DependsOn(local) => {
                let [assignment] = &*map[*local].usage else {
                    continue;
                };
                match assignment.node {
                    LocalUsage::Scalar(scalar) => all_assignments.push(scalar.clone()),
                    LocalUsage::Scalars(_) => {},
                    _ => return false,
                }
            },
            LocalUsage::DependsOnWithProj(local, base_proj) => {
                let [assignment] = &*map[*local].usage else {
                    continue;
                };
                match base_proj {
                    PlaceElem::Field(idx, _) if let LocalUsage::Scalars(scalars) = &assignment.node => {
                        all_assignments.push(scalars[idx.as_usize()].clone());
                    },
                    PlaceElem::Index(idx) if let LocalUsage::Scalars(scalars) = &assignment.node => {
                        all_assignments.push(scalars[idx.as_usize()].clone());
                    },
                    _ => return false,
                }
            },
        }
    }

    if let [head, tail @ ..] = &*all_assignments && tail.iter().all(|i| i == head) {
        return true;
    }

    false
}

fn nested_locals_are_not_from_bad_instr(map: &LocalUsageMap<'_>, usage: &LocalUsageValues<'_>) -> bool {
    // FIXME: This is a hacky workaround to not have a stack overflow. Instead, we should fix the root
    // cause.
    nested_locals_are_not_from_bad_instr_inner(map, usage, 0)
}

fn nested_locals_are_not_from_bad_instr_inner(
    map: &LocalUsageMap<'_>,
    usage: &LocalUsageValues<'_>,
    depth: usize,
) -> bool {
    if depth < 10 && !usage.is_from_bad_instr {
        let mut all_good_instrs = true;
        for assignment in &usage.usage {
            match assignment.node {
                LocalUsage::Scalar(_) | LocalUsage::Scalars(_) => continue,
                LocalUsage::DependsOn(local) | LocalUsage::DependsOnWithProj(local, _) => {
                    all_good_instrs &= nested_locals_are_not_from_bad_instr_inner(map, &map[local], depth + 1);
                },
            }
        }
        return all_good_instrs;
    }

    false
}
