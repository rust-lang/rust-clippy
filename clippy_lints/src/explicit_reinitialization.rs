use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use clippy_utils::{fn_has_unsatisfiable_preds, is_from_proc_macro};
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::graph::dominators::Dominators;
use rustc_data_structures::graph::iterate::DepthFirstSearch;
use rustc_data_structures::graph::WithSuccessors;
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{
    Closure, Expr, ExprKind, HirId, ImplItem, ImplItemKind, Item, ItemKind, Node, Path, PathSegment, QPath, StmtKind,
    TraitFn, TraitItem, TraitItemKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{self, BasicBlock, Body, Local, Location, Place, Statement, Terminator};
use rustc_session::{declare_lint_pass, declare_tool_lint, Session};
use rustc_span::Span;
use std::collections::BTreeSet;

declare_clippy_lint! {
    /// ### What it does
    /// If a reinitialization dominate all reachable usages, a fresh variable should be introduced
    ///
    /// ### Why is this bad?
    /// Introduce unnecessary mut.
    /// Not good in jumpping to definition in ide.
    ///
    /// ### Known Problems
    /// 1. Known false positive and false negative: see test
    /// 2. increase the peak memory usage
    /// ```
    /// let mut x = vec![1, 2, 3];
    /// x = vec![4, 5, 6];            // x is dropped here
    /// // let x = vec![4, 5, 6];     // x is no longer dropped here, but at the end of the scope
    /// ```
    ///
    /// ### Example
    /// ```rust
    /// let mut x = 1;
    /// println!("{x}");
    /// x = 2;
    /// println!("{x}");
    /// ```
    /// Use instead:
    /// ```rust
    /// let mut x = 1;
    /// println!("{x}");
    /// let x = 2;
    /// println!("{x}");
    /// ```
    #[clippy::version = "1.75.0"]
    pub EXPLICIT_REINITIALIZATION,
    style,
    "introduce a fresh variable instead of reinitialization"
}

declare_lint_pass!(ExplicitReinitialization => [EXPLICIT_REINITIALIZATION]);

impl<'tcx> LateLintPass<'tcx> for ExplicitReinitialization {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx rustc_hir::Stmt<'tcx>) {
        if stmt.span.from_expansion() || in_external_macro(cx.tcx.sess, stmt.span) {
            return;
        }

        for (parent_id, _) in cx.tcx.hir().parent_iter(stmt.hir_id) {
            let span = cx.tcx.hir().span(parent_id);
            if span.from_expansion() || in_external_macro(cx.tcx.sess, span) {
                return;
            }
        }

        let StmtKind::Semi(
            expr @ Expr {
                kind:
                    ExprKind::Assign(
                        Expr {
                            kind:
                                ExprKind::Path(QPath::Resolved(
                                    None,
                                    Path {
                                        segments: [PathSegment { args: None, .. }],
                                        ..
                                    },
                                )),
                            span: left_span,
                            ..
                        },
                        right,
                        _,
                    ),
                ..
            },
        ) = stmt.kind
        else {
            return;
        };
        if is_from_proc_macro(cx, expr) {
            return;
        }
        let Some(snip) = snippet_opt(cx, stmt.span) else {
            return;
        };
        let Some(local_def_id) = associated_fn(cx, stmt.hir_id) else {
            return;
        };
        let def_id = local_def_id.to_def_id();

        if fn_has_unsatisfiable_preds(cx, def_id) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id);
        let Some((_span, local, location)) = search_local(mir, *left_span, cx.tcx.sess) else {
            return;
        };
        let dominators = mir.basic_blocks.dominators();
        let Some((_span, start_location)) = search_mir_by_span(mir, right.span, dominators, cx.tcx.sess) else {
            return;
        };

        assert!(start_location.dominates(location, dominators));

        if dominate_all_usage(mir, dominators, local, start_location) {
            span_lint_and_sugg(
                cx,
                EXPLICIT_REINITIALIZATION,
                stmt.span,
                "create a fresh variable is more explicit",
                "create a fresh variable instead of reinitialization",
                format!("let mut {snip}"),
                Applicability::MachineApplicable,
            );
        }
    }
}

// based on associated_body()
fn associated_fn(cx: &LateContext<'_>, hir_id: HirId) -> Option<LocalDefId> {
    for (_hir_id, node) in cx.tcx.hir().parent_iter(hir_id) {
        match node {
            Node::Item(Item {
                owner_id,
                kind: ItemKind::Fn(.., _body),
                ..
            })
            | Node::TraitItem(TraitItem {
                owner_id,
                kind: TraitItemKind::Const(_, Some(_body)) | TraitItemKind::Fn(_, TraitFn::Provided(_body)),
                ..
            })
            | Node::ImplItem(ImplItem {
                owner_id,
                kind: ImplItemKind::Const(_, _body) | ImplItemKind::Fn(_, _body),
                ..
            }) => {
                return Some(owner_id.def_id);
            },

            Node::Item(Item {
                kind: ItemKind::Impl(..),
                ..
            })
            | Node::Expr(Expr {
                // abort if in any closure
                kind: ExprKind::Closure(Closure { .. }),
                ..
            }) => {
                return None;
            },
            _ => {},
        }
    }
    None
}

fn search_local(mir: &Body<'_>, left_span: Span, sess: &Session) -> Option<(Span, Local, Location)> {
    struct SmallestSpanVisitor<'c, 'a> {
        body: &'c Body<'a>,
        debug_local: FxHashSet<Local>,
        target_span: Span,
        sess: &'c Session,
        result: Option<(Span, Local, Location)>,
    }

    impl<'a, 'c> SmallestSpanVisitor<'a, 'c> {
        fn is_cleanup(&self, location: Location) -> bool {
            self.body.basic_blocks[location.block].is_cleanup
        }

        fn update(&mut self, span: Span, local: Local, location: Location) {
            if span.from_expansion() || in_external_macro(self.sess, span) {
                return;
            }
            if !span.contains(self.target_span) {
                return;
            }
            if !self.debug_local.contains(&local) {
                return;
            }
            if self.is_cleanup(location) {
                return;
            }
            if span.ctxt() != self.target_span.ctxt() {
                return;
            }
            match &self.result {
                Some((span_a, _, prev_locaion)) => match cmp_span(*span_a, span) {
                    SpanCmp::Eq => unreachable!("{:?} {:?} {:?}", span_a, prev_locaion, location),
                    SpanCmp::AContainB => {
                        self.result = Some((span, local, location));
                    },
                    SpanCmp::BContainA => {},
                    SpanCmp::Overlap | SpanCmp::NoOverLap => unreachable!(),
                },
                None => {
                    self.result = Some((span, local, location));
                },
            }
        }
    }

    impl<'tcx, 'a, 'c> Visitor<'tcx> for SmallestSpanVisitor<'a, 'c> {
        fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
            match &statement.kind {
                mir::StatementKind::Assign(box (Place { local, .. }, _)) | mir::StatementKind::StorageLive(local) => {
                    self.update(statement.source_info.span, *local, location);
                },
                _ => {},
            }
        }

        fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
            if let mir::TerminatorKind::Call { destination, .. } = &terminator.kind {
                self.update(terminator.source_info.span, destination.local, location);
            }
        }
    }

    let debug_local: FxHashSet<Local> = mir
        .var_debug_info
        .iter()
        .filter_map(|info| match &info.value {
            mir::VarDebugInfoContents::Place(Place { local, .. }) => Some(*local),
            mir::VarDebugInfoContents::Const(_) => None,
        })
        .collect();

    let mut accurate_visitor = SmallestSpanVisitor {
        body: mir,
        debug_local,
        target_span: left_span,
        sess,
        result: None,
    };
    accurate_visitor.visit_body(accurate_visitor.body);
    accurate_visitor.result
}

// must return Option bacause of expansion
fn search_mir_by_span(
    mir: &mir::Body<'_>,
    rvalue_span: Span,
    dominators: &Dominators<BasicBlock>,
    sess: &Session,
) -> Option<(Span, Location)> {
    struct SmallestSpanVisitor<'b, 'a> {
        body: &'b Body<'a>,
        dominators: &'b Dominators<BasicBlock>,
        target_span: Span,
        sess: &'b Session,
        result: Option<(Span, Location)>,
    }

    impl<'a, 'b> SmallestSpanVisitor<'a, 'b> {
        fn is_cleanup(&self, location: Location) -> bool {
            self.body.basic_blocks[location.block].is_cleanup
        }

        fn update(&mut self, span: Span, location: Location) {
            if span.from_expansion() || in_external_macro(self.sess, span) {
                return;
            }
            if !span.contains(self.target_span) {
                return;
            }
            if self.is_cleanup(location) {
                return;
            }
            if span.ctxt() != self.target_span.ctxt() {
                return;
            }
            match &self.result {
                Some((span_a, prev_location)) => match cmp_span(*span_a, span) {
                    SpanCmp::Eq => {
                        if prev_location.dominates(location, self.dominators) {
                            self.result = Some((span, location));
                        } else if location.dominates(*prev_location, self.dominators) {
                        } else {
                            unreachable!()
                        }
                    },
                    SpanCmp::AContainB => {
                        self.result = Some((span, location));
                    },
                    SpanCmp::BContainA => {},
                    SpanCmp::Overlap | SpanCmp::NoOverLap => unreachable!(),
                },
                None => {
                    self.result = Some((span, location));
                },
            }
        }
    }

    impl<'tcx, 'a, 'b> Visitor<'tcx> for SmallestSpanVisitor<'a, 'b> {
        fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
            if let mir::StatementKind::Assign(_) = &statement.kind {
                self.update(statement.source_info.span, location);
            }
        }

        fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
            if let mir::TerminatorKind::Call { .. } = &terminator.kind {
                self.update(terminator.source_info.span, location);
            }
        }
    }

    let mut accurate_visitor = SmallestSpanVisitor {
        body: mir,
        dominators,
        target_span: rvalue_span,
        sess,
        result: None,
    };
    accurate_visitor.visit_body(accurate_visitor.body);
    accurate_visitor.result
}

fn dominate_all_usage(
    mir: &mir::Body<'_>,
    dominators: &Dominators<BasicBlock>,
    local: Local,
    start_location: Location,
) -> bool {
    let mut dfs = DepthFirstSearch::new(&mir.basic_blocks);
    for successor in mir.basic_blocks.successors(start_location.block) {
        dfs.push_start_node(successor);
    }
    let reachable_bb: FxHashSet<BasicBlock> = dfs.collect();
    find_usage(mir, local)
        .into_iter()
        .filter(|location| reachable_bb.contains(&location.block))
        .filter(|location| !mir.basic_blocks[location.block].is_cleanup)
        .all(|location| start_location.dominates(location, dominators))
}

// copy from https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_borrowck/diagnostics/find_all_local_uses.rs.html#1-29
fn find_usage(body: &Body<'_>, local: Local) -> BTreeSet<Location> {
    struct AllLocalUsesVisitor {
        for_local: Local,
        uses: BTreeSet<Location>,
    }

    impl<'tcx> Visitor<'tcx> for AllLocalUsesVisitor {
        fn visit_local(&mut self, local: Local, _context: PlaceContext, location: Location) {
            if local == self.for_local {
                self.uses.insert(location);
            }
        }
    }

    let mut visitor = AllLocalUsesVisitor {
        for_local: local,
        uses: BTreeSet::default(),
    };
    visitor.visit_body(body);
    visitor.uses
}

#[derive(Debug, Copy, Clone)]
enum SpanCmp {
    Eq,
    AContainB,
    BContainA,
    Overlap,
    NoOverLap,
}

fn cmp_span(a: Span, b: Span) -> SpanCmp {
    if a == b {
        return SpanCmp::Eq;
    }
    if a.contains(b) {
        return SpanCmp::AContainB;
    }
    if b.contains(a) {
        return SpanCmp::BContainA;
    }
    if a.overlaps(b) {
        return SpanCmp::Overlap;
    }
    SpanCmp::NoOverLap
}
