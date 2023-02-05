use rustc_hir::{Expr, HirId};
use rustc_middle::mir::visit::{
    MutVisitor, MutatingUseContext, NonMutatingUseContext, PlaceContext, TyContext, Visitor,
};
use rustc_middle::mir::{
    traversal, Body, InlineAsmOperand, Local, LocalDecl, Location, Place, Statement, StatementKind, Terminator,
    TerminatorKind, START_BLOCK,
};
use rustc_middle::ty::{Region, Ty, TyCtxt};

mod possible_borrower;
pub use possible_borrower::PossibleBorrowerMap;

#[derive(Clone, Debug, Default)]
pub struct LocalUsage {
    /// The locations where the local is used, if any.
    pub local_use_locs: Vec<Location>,
    /// The locations where the local is consumed or mutated, if any.
    pub local_consume_or_mutate_locs: Vec<Location>,
}

pub fn visit_local_usage(locals: &[Local], mir: &Body<'_>, location: Location) -> Option<Vec<LocalUsage>> {
    let init = vec![
        LocalUsage {
            local_use_locs: Vec::new(),
            local_consume_or_mutate_locs: Vec::new(),
        };
        locals.len()
    ];

    traversal::ReversePostorder::new(mir, location.block).try_fold(init, |usage, (tbb, tdata)| {
        // Give up on loops
        if tdata.terminator().successors().any(|s| s == location.block) {
            return None;
        }

        let mut v = V {
            locals,
            location,
            results: usage,
        };
        v.visit_basic_block_data(tbb, tdata);
        Some(v.results)
    })
}

struct V<'a> {
    locals: &'a [Local],
    location: Location,
    results: Vec<LocalUsage>,
}

impl<'a, 'tcx> Visitor<'tcx> for V<'a> {
    fn visit_place(&mut self, place: &Place<'tcx>, ctx: PlaceContext, loc: Location) {
        if loc.block == self.location.block && loc.statement_index <= self.location.statement_index {
            return;
        }

        let local = place.local;

        for (i, self_local) in self.locals.iter().enumerate() {
            if local == *self_local {
                if !matches!(
                    ctx,
                    PlaceContext::MutatingUse(MutatingUseContext::Drop) | PlaceContext::NonUse(_)
                ) {
                    self.results[i].local_use_locs.push(loc);
                }
                if matches!(
                    ctx,
                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Move)
                        | PlaceContext::MutatingUse(MutatingUseContext::Borrow)
                ) {
                    self.results[i].local_consume_or_mutate_locs.push(loc);
                }
            }
        }
    }
}

/// Convenience wrapper around `visit_local_usage`.
pub fn used_exactly_once(mir: &rustc_middle::mir::Body<'_>, local: rustc_middle::mir::Local) -> Option<bool> {
    visit_local_usage(
        &[local],
        mir,
        Location {
            block: START_BLOCK,
            statement_index: 0,
        },
    )
    .map(|mut vec| {
        let LocalUsage { local_use_locs, .. } = vec.remove(0);
        local_use_locs
            .into_iter()
            .filter(|location| !is_local_assignment(mir, local, *location))
            .count()
            == 1
    })
}

/// Returns the `mir::Body` containing the node associated with `hir_id`.
#[allow(clippy::module_name_repetitions)]
pub fn enclosing_mir(tcx: TyCtxt<'_>, hir_id: HirId) -> &Body<'_> {
    let body_owner_local_def_id = tcx.hir().enclosing_body_owner(hir_id);
    tcx.optimized_mir(body_owner_local_def_id.to_def_id())
}

/// Tries to determine the `Local` corresponding to `expr`, if any.
/// This function is expensive and should be used sparingly.
pub fn expr_local(tcx: TyCtxt<'_>, expr: &Expr<'_>) -> Option<Local> {
    let mir = enclosing_mir(tcx, expr.hir_id);
    mir.local_decls.iter_enumerated().find_map(|(local, local_decl)| {
        if local_decl.source_info.span == expr.span {
            Some(local)
        } else {
            None
        }
    })
}

/// Tries to find the local in `to_mir` corresponding to `local` in `from_mir`.
pub fn translate_local<'tcx>(
    tcx: TyCtxt<'tcx>,
    from_mir: &Body<'tcx>,
    to_mir: &Body<'tcx>,
    local: Local,
) -> Option<Local> {
    let equiv_decl = |lhs: &LocalDecl<'tcx>, rhs: &LocalDecl<'tcx>| {
        lhs.mutability == rhs.mutability
            && tcx.erase_regions_ty(lhs.ty) == tcx.erase_regions_ty(rhs.ty)
            && lhs.source_info.span == rhs.source_info.span
    };

    let from_decl = &from_mir.local_decls[local];

    // Fast path
    if to_mir
        .local_decls
        .get(local)
        .map_or(false, |to_decl| equiv_decl(from_decl, to_decl))
    {
        return Some(local);
    }

    // Slow path
    to_mir
        .local_decls
        .iter()
        .position(|to_decl| equiv_decl(from_decl, to_decl))
        .map(Into::into)
}

/// Tries to find the location in `to_mir` corresponding to `location` in `from_mir`.
pub fn translate_location<'tcx>(
    tcx: TyCtxt<'tcx>,
    from_mir: &Body<'tcx>,
    to_mir: &Body<'tcx>,
    location: Location,
) -> Option<Location> {
    let normalized_lhs = from_mir
        .stmt_at(location)
        .map_left(|statement| normalize_statement(tcx, statement))
        .map_right(|terminator| normalize_terminator(tcx, terminator));

    for (block, block_data) in to_mir.basic_blocks.iter_enumerated() {
        if let Some(location) = normalized_lhs.as_ref().either(
            |normalized_lhs| {
                (0..block_data.statements.len()).find_map(|statement_index| {
                    let rhs = &block_data.statements[statement_index];
                    if normalized_lhs.source_info.span == rhs.source_info.span
                        && normalized_lhs.kind == normalize_statement(tcx, rhs).kind
                    {
                        Some(Location { block, statement_index })
                    } else {
                        None
                    }
                })
            },
            |normalized_lhs| {
                if block_data.terminator.as_ref().map_or(false, |rhs| {
                    normalized_lhs.source_info.span == rhs.source_info.span
                        && normalized_lhs.kind == normalize_terminator(tcx, rhs).kind
                }) {
                    Some(Location {
                        block,
                        statement_index: block_data.statements.len(),
                    })
                } else {
                    None
                }
            },
        ) {
            return Some(location);
        }
    }

    None
}

fn normalize_statement<'tcx>(tcx: TyCtxt<'tcx>, statement: &Statement<'tcx>) -> Statement<'tcx> {
    let mut statement = statement.clone();
    Normalizer { tcx }.visit_statement(&mut statement, Location::START);
    statement
}

fn normalize_terminator<'tcx>(tcx: TyCtxt<'tcx>, terminator: &Terminator<'tcx>) -> Terminator<'tcx> {
    let mut terminator = terminator.clone();
    Normalizer { tcx }.visit_terminator(&mut terminator, Location::START);
    match terminator.kind {
        // No basic blocks
        TerminatorKind::Abort
        | TerminatorKind::GeneratorDrop
        | TerminatorKind::Resume
        | TerminatorKind::Return
        | TerminatorKind::Unreachable => {},

        // One basic block
        TerminatorKind::Goto { ref mut target } => {
            *target = Location::START.block;
        },

        // Two basic blocks
        TerminatorKind::FalseEdge {
            ref mut real_target,
            ref mut imaginary_target,
        } => {
            *real_target = Location::START.block;
            *imaginary_target = Location::START.block;
        },

        // One optional and one non-optional basic block
        TerminatorKind::Assert {
            ref mut target,
            cleanup: ref mut unwind,
            ..
        }
        | TerminatorKind::Drop {
            ref mut target,
            ref mut unwind,
            ..
        }
        | TerminatorKind::DropAndReplace {
            ref mut target,
            ref mut unwind,
            ..
        }
        | TerminatorKind::FalseUnwind {
            real_target: ref mut target,
            ref mut unwind,
            ..
        }
        | TerminatorKind::Yield {
            resume: ref mut target,
            drop: ref mut unwind,
            ..
        } => {
            *target = Location::START.block;
            *unwind = None;
        },

        // Two optional basic blocks
        TerminatorKind::Call {
            ref mut target,
            ref mut cleanup,
            ..
        }
        | TerminatorKind::InlineAsm {
            destination: ref mut target,
            ref mut cleanup,
            ..
        } => {
            *target = None;
            *cleanup = None;
        },

        // Arbitrarily many basic blocks
        TerminatorKind::SwitchInt { ref mut targets, .. } => {
            for (_, ref mut target) in targets.iter() {
                *target = Location::START.block;
            }
        },
    }
    terminator
}

struct Normalizer<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> MutVisitor<'tcx> for Normalizer<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_local(&mut self, local: &mut Local, _context: PlaceContext, _location: Location) {
        *local = Local::from_u32(0);
    }

    fn visit_region(&mut self, region: &mut Region<'tcx>, _: Location) {
        *region = self.tcx.lifetimes.re_erased;
    }

    fn visit_ty(&mut self, ty: &mut Ty<'tcx>, _: TyContext) {
        *ty = self.tcx.erase_regions_ty(*ty);
    }
}

/// Returns a vector of `mir::Location` where `local` is assigned.
pub fn local_assignments(mir: &Body<'_>, local: Local) -> Vec<Location> {
    let mut locations = Vec::new();
    for (block, data) in mir.basic_blocks.iter_enumerated() {
        for statement_index in 0..=data.statements.len() {
            let location = Location { block, statement_index };
            if is_local_assignment(mir, local, location) {
                locations.push(location);
            }
        }
    }
    locations
}

// `is_local_assignment` is based on `is_place_assignment`:
// https://github.com/rust-lang/rust/blob/b7413511dc85ec01ef4b91785f86614589ac6103/compiler/rustc_middle/src/mir/visit.rs#L1350
fn is_local_assignment(mir: &Body<'_>, local: Local, location: Location) -> bool {
    let Location { block, statement_index } = location;
    let basic_block = &mir.basic_blocks[block];
    if statement_index < basic_block.statements.len() {
        let statement = &basic_block.statements[statement_index];
        if let StatementKind::Assign(box (place, _)) = statement.kind {
            place.as_local() == Some(local)
        } else {
            false
        }
    } else {
        let terminator = basic_block.terminator();
        match &terminator.kind {
            TerminatorKind::Call { destination, .. } => destination.as_local() == Some(local),
            TerminatorKind::InlineAsm { operands, .. } => operands.iter().any(|operand| {
                if let InlineAsmOperand::Out { place: Some(place), .. } = operand {
                    place.as_local() == Some(local)
                } else {
                    false
                }
            }),
            _ => false,
        }
    }
}
