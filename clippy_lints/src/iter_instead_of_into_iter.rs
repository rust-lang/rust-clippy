use clippy_utils::diagnostics;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::{intravisit::FnKind, Body, FnDecl, HirId};
use rustc_index::bit_set::HybridBitSet;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    mir::{
        self,
        visit::{PlaceContext, Visitor as MirVisitor},
        BasicBlock, Local, Location, Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
    },
    ty,
};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.60.0"]
    pub ITER_INSTEAD_OF_INTO_ITER,
    style,
    "default lint description"
}
declare_lint_pass!(IterInsteadOfIntoIter => [ITER_INSTEAD_OF_INTO_ITER]);

impl<'tcx> LateLintPass<'tcx> for IterInsteadOfIntoIter {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _: Span,
        _: HirId,
    ) {
        let def_id = cx.tcx.hir().body_owner_def_id(body.id());

        if clippy_utils::fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id.to_def_id());

        let mut iter_visitor = IterVisitor {
            mir,
            cx,
            iter_calls: Vec::new(),
        };
        iter_visitor.visit_body(mir);
        let mut iter_calls = iter_visitor.iter_calls;

        if iter_calls.is_empty() {
            return;
        }

        let origins = find_origins(cx, mir, &iter_calls);

        if origins.is_empty() {
            return;
        }

        for (k, v) in &origins {
            for index in &v.indexes {
                iter_calls[*index].origin = Some(*k);
            }
        }

        iter_calls.retain(|x| x.origin.is_some());

        let mut re_vistor = TransitiveRelationVisitor::default();
        re_vistor.visit_body(mir);

        let origins: FxHashMap<_, _> = origins
            .keys()
            .map(|k| (*k, re_vistor.relations.reachable_from(*k, mir.local_decls.len())))
            .collect();

        let mut used_after_call_visitor = UseAfterIterVisitor {
            calls: iter_calls,
            origins,
            relations: re_vistor.relations,
            domain_size: mir.local_decls.len(),
        };

        used_after_call_visitor.visit_body(mir);

        for call in used_after_call_visitor
            .calls
            .iter()
            .filter(|x| !x.origin_used_after_call)
        {
            let span = mir.basic_blocks()[call.location.block].terminator().source_info.span;
            diagnostics::span_lint_and_sugg(
                cx,
                ITER_INSTEAD_OF_INTO_ITER,
                span,
                &format!(
                    "this `.iter()` call can be replaced with `.into_iter()` as the container is not used afterwards",
                ),
                "replace `iter()` with",
                "into_iter()".to_string(),
                Applicability::MachineApplicable,
            );
        }

        // println!("MIR:");
        // println!();
        // dbg!(&mir);
    }
}

#[derive(Debug)]
struct IterCall {
    local: Local,
    location: Location,
    dest: Local,
    origin_used_after_call: bool,
    origin: Option<Local>,
}

type Origins = FxHashMap<Local, Origin>;

impl IterCall {
    fn new(local: Local, location: Location, dest: Local) -> IterCall {
        IterCall {
            local,
            location,
            dest,
            origin_used_after_call: false,
            origin: None,
        }
    }
}

struct IterVisitor<'a, 'tcx> {
    mir: &'a rustc_middle::mir::Body<'tcx>,
    cx: &'a LateContext<'tcx>,
    iter_calls: Vec<IterCall>,
}
struct Origin {
    indexes: Vec<usize>,
    found: bool,
}

impl Origin {
    fn new(index: usize) -> Origin {
        Origin {
            indexes: vec![index],
            found: false,
        }
    }
}

// collect info for all calls of the kind ```foo.iter()```
impl<'a, 'txc> MirVisitor<'txc> for IterVisitor<'a, 'txc> {
    fn visit_terminator(&mut self, term: &Terminator<'txc>, loc: Location) {
        if let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &term.kind
        {
            let ty = func.ty(self.mir, self.cx.tcx);
            if let ty::FnDef(def_id, _) = ty.kind() {
                let defpath = self.cx.tcx.def_path(*def_id);
                let name = defpath.data.last().unwrap().data.get_opt_name();
                if let Some(sym::iter) = name {
                    // Could it be something else than a move?
                    if let Operand::Move(place) = args[0] {
                        if let Some(dest) = destination {
                            if !is_inside_loop(self.mir, term, loc) {
                                self.iter_calls.push(IterCall::new(place.local, loc, dest.0.local));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn is_inside_loop<'tcx>(mir: &mir::Body<'tcx>, term: &mir::Terminator<'tcx>, loc: Location) -> bool {
    let mut seen = HybridBitSet::new_empty(mir.basic_blocks().len());
    let mut stack: Vec<BasicBlock> = term.successors().map(|bb| *bb).collect();
    while let Some(bb) = stack.pop() {
        if bb == loc.block {
            return true;
        }
        if seen.insert(bb.index()) {
            if let Some(next_term) = &mir.basic_blocks()[bb].terminator {
                for successor in next_term.successors() {
                    stack.push(*successor);
                }
            }
        }
    }
    false
}

// find the origin of each iter usage
fn find_origins<'tcx>(cx: &LateContext<'tcx>, mir: &mir::Body<'tcx>, iter_calls: &Vec<IterCall>) -> Origins {
    let mut origins: Origins = iter_calls
        .iter()
        .enumerate()
        .map(|(i, v)| (v.local, Origin::new(i)))
        .collect();

    let mut find_origin = |src, dest| {
        if let Some(mut origin) = origins.remove(dest) {
            let entry = origins
                .entry(src)
                .and_modify(|o| o.indexes.append(&mut origin.indexes))
                .or_insert(origin);
            // consider origin found when a local with local info is found
            if let Some(_) = mir.local_decls[src].local_info {
                let ty = mir.local_decls[src].ty;
                // only interest in owned origins
                if let ty::Ref(..) = ty.kind() {
                    return;
                }
                // only consider owned types that have a valid iter method
                if clippy_utils::ty::has_iter_method(cx, ty).is_none() {
                    return;
                }
                entry.found = true;
            }
        }
    };

    for (bb, bbdata) in mir.basic_blocks().iter_enumerated().rev() {
        if let Some(term) = &bbdata.terminator {
            if let TerminatorKind::Call { destination, args, .. } = &term.kind {
                if let Some((dest, _)) = destination {
                    if let Some(arg) = args.first() && args.len() == 1 {
                        if let Operand::Move(src) = arg {
                            if !bbdata.terminator().successors().any(|x| *x == bb) {
                                find_origin(src.local, &dest.local);
                            }
                        }
                    }
                }
            }
        }

        for stmt in bbdata.statements.iter().rev() {
            if let StatementKind::Assign(box (dest, rvalue)) = &stmt.kind {
                if let Rvalue::Ref(_, _, src) = rvalue {
                    find_origin(src.local, &dest.local);
                }
                // Should I consider other variants here?
            }
        }
    }
    origins.retain(|_, v| v.found);
    origins
}

#[derive(Default)]
struct TransitiveRelationVisitor {
    relations: TransitiveRelation,
}

impl TransitiveRelationVisitor {
    fn process_local(&mut self, lhs: Local, rhs: Local, _: Location) {
        self.relations.add(rhs, lhs);
    }
    fn process_operand<'tcx>(&mut self, lhs: Local, oper: &Operand<'tcx>, location: Location) {
        match oper {
            Operand::Copy(place) | Operand::Move(place) => self.process_local(lhs, place.local, location),
            Operand::Constant(..) => (),
        }
    }
}

impl<'txc> MirVisitor<'txc> for TransitiveRelationVisitor {
    fn visit_assign(&mut self, place: &Place<'txc>, rvalue: &Rvalue<'txc>, location: Location) {
        let lhs = place.local;
        match rvalue {
            Rvalue::Ref(_, _, place) => self.process_local(lhs, place.local, location),
            Rvalue::Cast(_, oper, _) => self.process_operand(lhs, oper, location),
            Rvalue::BinaryOp(_, box (a, b)) => {
                self.process_operand(lhs, a, location);
                self.process_operand(lhs, b, location);
            },
            Rvalue::UnaryOp(_, oper) => self.process_operand(lhs, oper, location),
            Rvalue::Aggregate(_, opers) => {
                for oper in opers {
                    self.process_operand(lhs, oper, location)
                }
            },
            _ => (),
        }
    }

    fn visit_terminator(&mut self, term: &Terminator<'txc>, location: Location) {
        if let TerminatorKind::Call { args, destination, .. } = &term.kind {
            let lhs = if let Some(d) = destination {
                d.0.local
            } else {
                return;
            };
            for arg in args {
                self.process_operand(lhs, arg, location);
            }
        }
    }
}
struct UseAfterIterVisitor {
    calls: Vec<IterCall>,
    origins: FxHashMap<Local, HybridBitSet<Local>>,
    relations: TransitiveRelation,
    domain_size: usize,
}

impl<'tcx> MirVisitor<'tcx> for UseAfterIterVisitor {
    fn visit_place(&mut self, place: &Place<'tcx>, _: PlaceContext, location: Location) {
        for call in self.calls.iter_mut().filter(|x| !x.origin_used_after_call) {
            if location > call.location {
                let origin_refs = self.origins.get(&call.origin.unwrap()).unwrap();
                if place.local != call.dest
                    && origin_refs.contains(place.local)
                    && !self.relations.is_descendant(call.dest, place.local, self.domain_size)
                {
                    call.origin_used_after_call = true;
                }
            }
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<'tcx>, location: Location) {
        if let StatementKind::StorageLive(_) | StatementKind::StorageDead(_) = stmt.kind {
            return;
        }
        self.super_statement(stmt, location);
    }

    fn visit_terminator(&mut self, term: &Terminator<'tcx>, location: Location) {
        if let TerminatorKind::Drop { .. } = term.kind {
            return;
        }
        self.super_terminator(term, location);
    }
}

#[derive(Debug, Default)]
struct TransitiveRelation {
    relations: FxHashMap<Local, Vec<Local>>,
}
impl TransitiveRelation {
    fn add(&mut self, a: Local, b: Local) {
        self.relations.entry(a).or_default().push(b);
    }

    fn reachable_from(&self, a: Local, domain_size: usize) -> HybridBitSet<Local> {
        let mut seen = HybridBitSet::new_empty(domain_size);
        let mut stack = vec![a];
        while let Some(u) = stack.pop() {
            if let Some(edges) = self.relations.get(&u) {
                for &v in edges {
                    if seen.insert(v) {
                        stack.push(v);
                    }
                }
            }
        }
        seen
    }

    fn is_descendant(&self, a: Local, b: Local, domain_size: usize) -> bool {
        let mut seen = HybridBitSet::new_empty(domain_size);
        let mut stack = vec![a];
        while let Some(u) = stack.pop() {
            if let Some(edges) = self.relations.get(&u) {
                for &v in edges {
                    if v == b {
                        return true;
                    }
                    if seen.insert(v) {
                        stack.push(v);
                    }
                }
            }
        }
        false
    }
}
