use clippy_utils::consts::is_promotable;
use clippy_utils::diagnostics::{span_lint_and_note, span_lint_hir_and_then};
use clippy_utils::mir::{location_to_node, StatementOrTerminator};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, BorrowKind, Expr, ExprKind, FnDecl, HirId, ItemKind, OwnerNode};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::visit::{MutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{
    self, BasicBlock, BasicBlockData, CallSource, Local, Location, Place, PlaceRef, ProjectionElem, Rvalue, SourceInfo,
    StatementKind, TerminatorKind,
};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for raw pointers pointing to temporary values that will **not** be promoted to a
    /// constant through
    /// [constant promotion](https://doc.rust-lang.org/stable/reference/destructors.html#constant-promotion).
    ///
    /// ### Why is this bad?
    /// Usage of such a pointer will result in Undefined Behavior, as the pointer will stop
    /// pointing to valid stack memory once the temporary is dropped.
    ///
    /// ### Known problems
    /// Expects any call to methods named `as_ptr` or `as_mut_ptr` returning a raw pointer to have
    /// that raw pointer point to data owned by self. Essentially, it will lint all temporary
    /// `as_ptr` calls even if the pointer doesn't point to the temporary.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn returning_temp() -> *const i32 {
    ///     let x = 0;
    ///     &x as *const i32
    /// }
    ///
    /// let px = returning_temp();
    /// unsafe { *px }; // ⚠️
    /// let pv = vec![].as_ptr();
    /// unsafe { *pv }; // ⚠️
    /// ```
    #[clippy::version = "1.72.0"]
    pub PTR_TO_TEMPORARY,
    // TODO: Let's make it warn-by-default for now, and change this to deny-by-default once we know
    // there are no major FPs
    suspicious,
    "disallows obtaining raw pointers to temporary values"
}
declare_lint_pass!(PtrToTemporary => [PTR_TO_TEMPORARY]);

impl<'tcx> LateLintPass<'tcx> for PtrToTemporary {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        check_for_returning_raw_ptr(cx, expr);
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'_>,
        _: &FnDecl<'_>,
        _: &Body<'_>,
        _: Span,
        def_id: LocalDefId,
    ) {
        let mir = cx.tcx.optimized_mir(def_id);

        // Collect all local assignments in this body. This is faster than continuously passing over the
        // body every time we want to get the assignments.
        let mut assignments = LocalAssignmentsVisitor {
            results: FxHashMap::default(),
        };
        assignments.visit_body(mir);

        let mut v = DanglingPtrVisitor {
            cx,
            body: mir,
            results: vec![],
            local_assignments: assignments.results,
        };
        v.visit_body(mir);

        for (span, hir_id, ident) in v.results {
            // TODO: We need to lint on the call in question instead, so lint attributes work fine. I'm not sure
            // how to though
            span_lint_hir_and_then(
                cx,
                PTR_TO_TEMPORARY,
                hir_id,
                span,
                &format!("calling `{ident}` on a temporary value"),
                |diag| {
                    diag.note(
                        "usage of this pointer will cause Undefined Behavior as the temporary will be deallocated at \
                         the end of the statement, yet the pointer will continue pointing to it, resulting in a \
                         dangling pointer",
                    );
                },
            );
        }
    }
}

/// Check for returning raw pointers to temporaries that are not promoted to a constant
fn check_for_returning_raw_ptr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    // Get the final return statement if this is a return statement, or don't lint
    let expr = if let ExprKind::Ret(Some(expr)) = expr.kind {
        expr
    } else if let OwnerNode::Item(parent) = cx.tcx.hir().owner(cx.tcx.hir().get_parent_item(expr.hir_id))
        && let ItemKind::Fn(_, _, body) = parent.kind
        && let block = cx.tcx.hir().body(body).value
        && let ExprKind::Block(block, _) = block.kind
        && let Some(final_block_expr) = block.expr
        && final_block_expr.hir_id == expr.hir_id
    {
        expr
    } else {
        return false;
    };

    if let ExprKind::Cast(cast_expr, _) = expr.kind
        && let ExprKind::AddrOf(BorrowKind::Ref, _, e) = cast_expr.kind
        && !is_promotable(cx, e)
    {
        span_lint_and_note(
            cx,
            PTR_TO_TEMPORARY,
            expr.span,
            "returning a raw pointer to a temporary value that cannot be promoted to a constant",
            None,
            "usage of this pointer by callers will cause Undefined Behavior as the temporary will be deallocated at \
             the end of the statement, yet the pointer will continue pointing to it, resulting in a dangling pointer",
        );

        return true;
    }

    false
}

struct LocalAssignmentsVisitor {
    results: FxHashMap<Local, Vec<Location>>,
}

impl Visitor<'_> for LocalAssignmentsVisitor {
    fn visit_place(&mut self, place: &Place<'_>, ctxt: PlaceContext, loc: Location) {
        if matches!(
            ctxt,
            PlaceContext::MutatingUse(
                MutatingUseContext::Store | MutatingUseContext::Call | MutatingUseContext::Borrow
            )
        ) {
            self.results.entry(place.local).or_insert(vec![]).push(loc);
        }
    }
}

struct DanglingPtrVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    body: &'tcx mir::Body<'tcx>,
    local_assignments: FxHashMap<Local, Vec<Location>>,
    results: Vec<(Span, HirId, Symbol)>,
}

impl<'tcx> Visitor<'tcx> for DanglingPtrVisitor<'_, 'tcx> {
    fn visit_basic_block_data(&mut self, _: BasicBlock, data: &BasicBlockData<'tcx>) {
        let Self {
            cx,
            body,
            local_assignments,
            results,
        } = self;

        if let Some(term) = &data.terminator
            && let TerminatorKind::Call {
                func,
                args,
                destination,
                target: Some(target),
                call_source: CallSource::Normal,
                ..
            } = &term.kind
            && destination.ty(&body.local_decls, cx.tcx).ty.is_unsafe_ptr()
            && let [recv] = args.as_slice()
            && let Some(recv) = recv.place()
            && let Some((def_id, _)) = func.const_fn_def()
            && let Some(ident) = returns_ptr_to_self(cx, def_id)
            && let Ok(recv) = traverse_up_until_owned(body, local_assignments, recv)
        {

            check_for_dangling(
                body,
                *target,
                recv,
                destination.as_ref(),
                term.source_info,
                ident,
                results,
            );
        }
    }
}

fn check_for_dangling<'tcx>(
    body: &mir::Body<'tcx>,
    bb: BasicBlock,
    mut recv: PlaceRef<'tcx>,
    ptr: PlaceRef<'_>,
    source_info: SourceInfo,
    ident: Symbol,
    results: &mut Vec<(Span, HirId, Symbol)>,
) {
    let data = &body.basic_blocks[bb];
    let mut recv_dead = false;

    // If there's a `Drop`, we must include the statements in its target so we don't miss any
    // potentially important `StorageDead`s.
    let rest = vec![];
    let rest = if let Some(term) = &data.terminator && let TerminatorKind::Drop { place, target, .. } = term.kind {
        // This indicates a bug in our heuristic. It's normally fine if the `Drop` is present, but
        // if it isn't (i.e., no drop glue) then we may have FNs, or worse. Let's catch this early
        // if there are upstream MIR changes.
        debug_assert_eq!(place.as_ref(), recv, "dropped place is not receiver");
        // In release mode, let's prevent a few FPs where `Drop` is present.
        recv = place.as_ref();

        &body.basic_blocks[target].statements
    } else {
        &rest
    };

    for dead_local in data.statements.iter().chain(rest).filter_map(|stmt| {
        if let StatementKind::StorageDead(local) = stmt.kind {
            return Some(local);
        }

        None
    }) {
        match (dead_local == recv.local, dead_local == ptr.local) {
            (true, false) => recv_dead = true,
            (false, true) if recv_dead => {
                results.push((
                    source_info.span,
                    body.source_scopes[source_info.scope]
                        .local_data
                        .clone()
                        .assert_crate_local()
                        .lint_root,
                    ident,
                ));
            },
            _ => continue,
        }
    }
}

/// Traverses the MIR backwards until it finds owned data. This can be assumed to be the dropped
/// data in the next `Drop` terminator, if not this indicates a bug in our heuristic.
fn traverse_up_until_owned<'tcx>(
    body: &'tcx mir::Body<'tcx>,
    local_assignments: &FxHashMap<Local, Vec<Location>>,
    start: Place<'tcx>,
) -> Result<PlaceRef<'tcx>, TraverseError> {
    traverse_up_until_owned_inner(body, local_assignments, start.as_ref(), 0)
}

fn traverse_up_until_owned_inner<'tcx>(
    body: &'tcx mir::Body<'tcx>,
    local_assignments: &FxHashMap<Local, Vec<Location>>,
    current_place: PlaceRef<'tcx>,
    depth: usize,
) -> Result<PlaceRef<'tcx>, TraverseError> {
    if depth > 100 {
        return Err(TraverseError::MaxDepthReached);
    }
    let Some(current) = local_assignments.get(&current_place.local) else {
        return Err(TraverseError::NoAssignments);
    };
    if current.is_empty() {
        return Err(TraverseError::NoAssignments);
    }
    let [current] = current.as_slice() else {
        return Err(TraverseError::TooManyAssignments);
    };
    let current = location_to_node(body, *current);
    let next = match current {
        StatementOrTerminator::Statement(stmt) if let StatementKind::Assign(box (_, rvalue)) = &stmt.kind => {
            match rvalue {
                Rvalue::Use(op) | Rvalue::Cast(_, op, _) => {
                    let Some(place) = op.place() else {
                        return Err(TraverseError::LikelyPromoted);
                    };
                    // If there's a field access, this is likely to be accessing `.0` on a
                    // `Unique`. We need a better heuristic for this though, as this may lead to
                    // some FPs.
                    if let Some(place) = place.iter_projections().find_map(|proj| {
                        if matches!(proj.1, ProjectionElem::Field(_, _)) {
                            return Some(proj.0);
                        }
                        None
                    }) {
                        return Ok(place);
                    }
                    place.as_ref()
                }
                Rvalue::Ref(_, _, place) => {
                    if !place.has_deref() {
                        return Ok(place.as_ref());
                    }
                    place.as_ref()
                }
                // Give up if we can't determine it's dangling with near 100% accuracy
                _ => return Err(TraverseError::InvalidOp),
            }
        }
        StatementOrTerminator::Terminator(term) if let TerminatorKind::Call { args, .. } = &term.kind
            && let [arg] = args.as_slice() =>
        {
            let Some(place) = arg.place() else {
                return Err(TraverseError::LikelyPromoted);
            };
            place.as_ref()
        }
        // Give up if we can't determine it's dangling with near 100% accuracy
        _ => return Err(TraverseError::InvalidOp),
    };

    traverse_up_until_owned_inner(body, local_assignments, next, depth + 1)
}

enum TraverseError {
    NoAssignments,
    TooManyAssignments,
    MaxDepthReached,
    LikelyPromoted,
    InvalidOp,
}

/// Whether the call returns a raw pointer to data owned by self, i.e., `as_ptr` and friends. If so,
/// if it's temporary it will be dangling and we should lint it. Returns the name of the call if so.
fn returns_ptr_to_self(cx: &LateContext<'_>, def_id: DefId) ->Option<Symbol> {
    let path = cx.tcx.def_path(def_id).data;

    if let [.., last] = &*path
        && let Some(ident) = last.data.get_opt_name()
        && (ident == sym::as_ptr || ident == sym!(as_mut_ptr))
    {
        return Some(ident);
    }

    // TODO: More checks here. We want to lint most libstd functions that return a pointer that
    // aren't named `as_ptr`.
    None
}
