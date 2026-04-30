use crate::macros::root_macro_call_first_node;
use crate::res::MaybeResPath;
use crate::visitors::{Descend, Visitable, for_each_expr, for_each_expr_without_closures};
use crate::{self as utils, get_enclosing_loop_or_multi_call_closure, sym};
use core::ops::ControlFlow;
use hir::def::Res;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::intravisit::{self, Visitor, walk_block, walk_expr, walk_path, walk_stmt};
use rustc_hir::{self as hir, Block, Expr, ExprKind, HirId, HirIdSet, Pat, Path, Stmt, StmtKind};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, Place, PlaceBase, PlaceWithHirId};
use rustc_lint::LateContext;
use rustc_middle::hir::nested_filter;
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty;
use rustc_span::Symbol;

/// Returns a set of mutated local variable IDs, or `None` if mutations could not be determined.
pub fn mutated_variables<'tcx>(expr: &'tcx Expr<'_>, cx: &LateContext<'tcx>) -> Option<HirIdSet> {
    let mut delegate = MutVarsDelegate {
        used_mutably: HirIdSet::default(),
        skip: false,
    };
    ExprUseVisitor::for_clippy(cx, expr.hir_id.owner.def_id, &mut delegate)
        .walk_expr(expr)
        .into_ok();

    if delegate.skip {
        return None;
    }
    Some(delegate.used_mutably)
}

pub fn is_potentially_mutated<'tcx>(variable: HirId, expr: &'tcx Expr<'_>, cx: &LateContext<'tcx>) -> bool {
    mutated_variables(expr, cx).is_none_or(|mutated| mutated.contains(&variable))
}

pub fn is_potentially_local_place(local_id: HirId, place: &Place<'_>) -> bool {
    match place.base {
        PlaceBase::Local(id) => id == local_id,
        PlaceBase::Upvar(_) => {
            // Conservatively assume yes.
            true
        },
        _ => false,
    }
}

struct MutVarsDelegate {
    used_mutably: HirIdSet,
    skip: bool,
}

impl MutVarsDelegate {
    fn update(&mut self, cat: &PlaceWithHirId<'_>) {
        match cat.place.base {
            PlaceBase::Local(id) => {
                self.used_mutably.insert(id);
            },
            PlaceBase::Upvar(_) => {
                //FIXME: This causes false negatives. We can't get the `NodeId` from
                //`Categorization::Upvar(_)`. So we search for any `Upvar`s in the
                //`while`-body, not just the ones in the condition.
                self.skip = true;
            },
            _ => {},
        }
    }
}

impl<'tcx> Delegate<'tcx> for MutVarsDelegate {
    fn consume(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn borrow(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId, bk: ty::BorrowKind) {
        if bk == ty::BorrowKind::Mutable {
            self.update(cmt);
        }
    }

    fn mutate(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId) {
        self.update(cmt);
    }

    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}

pub struct ParamBindingIdCollector {
    pub binding_hir_ids: Vec<HirId>,
}
impl<'tcx> ParamBindingIdCollector {
    fn collect_binding_hir_ids(body: &'tcx hir::Body<'tcx>) -> Vec<HirId> {
        let mut hir_ids: Vec<HirId> = Vec::new();
        for param in body.params {
            let mut finder = ParamBindingIdCollector {
                binding_hir_ids: Vec::new(),
            };
            finder.visit_param(param);
            for hir_id in &finder.binding_hir_ids {
                hir_ids.push(*hir_id);
            }
        }
        hir_ids
    }
}
impl<'tcx> Visitor<'tcx> for ParamBindingIdCollector {
    fn visit_pat(&mut self, pat: &'tcx Pat<'tcx>) {
        if let hir::PatKind::Binding(_, hir_id, ..) = pat.kind {
            self.binding_hir_ids.push(hir_id);
        }
        intravisit::walk_pat(self, pat);
    }
}

pub struct BindingUsageFinder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    binding_ids: Vec<HirId>,
}
impl<'a, 'tcx> BindingUsageFinder<'a, 'tcx> {
    pub fn are_params_used(cx: &'a LateContext<'tcx>, body: &'tcx hir::Body<'tcx>) -> bool {
        let mut finder = BindingUsageFinder {
            cx,
            binding_ids: ParamBindingIdCollector::collect_binding_hir_ids(body),
        };
        finder.visit_body(body).is_break()
    }
}
impl<'tcx> Visitor<'tcx> for BindingUsageFinder<'_, 'tcx> {
    type Result = ControlFlow<()>;
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) -> Self::Result {
        if let Res::Local(id) = path.res
            && self.binding_ids.contains(&id)
        {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}

/// Checks if the given expression is a macro call to `todo!()` or `unimplemented!()`.
pub fn is_todo_unimplemented_macro(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    root_macro_call_first_node(cx, expr)
        .and_then(|macro_call| cx.tcx.get_diagnostic_name(macro_call.def_id))
        .is_some_and(|macro_name| matches!(macro_name, sym::todo_macro | sym::unimplemented_macro))
}

/// Checks if the given expression is a stub, i.e., a `todo!()` or `unimplemented!()` expression,
/// or a block whose last expression is a `todo!()` or `unimplemented!()`.
pub fn is_todo_unimplemented_stub(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::Block(block, _) = expr.kind {
        if let Some(last_expr) = block.expr {
            return is_todo_unimplemented_macro(cx, last_expr);
        }

        return block.stmts.last().is_some_and(|stmt| {
            if let StmtKind::Expr(expr) | StmtKind::Semi(expr) = stmt.kind {
                return is_todo_unimplemented_macro(cx, expr);
            }
            false
        });
    }

    is_todo_unimplemented_macro(cx, expr)
}

/// Checks if the given expression contains macro call to `todo!()` or `unimplemented!()`.
pub fn contains_todo_unimplement_macro(cx: &LateContext<'_>, expr: &'_ Expr<'_>) -> bool {
    for_each_expr_without_closures(expr, |e| {
        if is_todo_unimplemented_macro(cx, e) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}

pub fn contains_return_break_continue_macro(expression: &Expr<'_>) -> bool {
    for_each_expr_without_closures(expression, |e| {
        match e.kind {
            ExprKind::Ret(..) | ExprKind::Break(..) | ExprKind::Continue(..) => ControlFlow::Break(()),
            // Something special could be done here to handle while or for loop
            // desugaring, as this will detect a break if there's a while loop
            // or a for loop inside the expression.
            _ if e.span.from_expansion() => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    })
    .is_some()
}

pub fn local_used_in<'tcx>(cx: &LateContext<'tcx>, local_id: HirId, v: impl Visitable<'tcx>) -> bool {
    for_each_expr(cx, v, |e| {
        if e.res_local_id() == Some(local_id) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}

pub fn local_used_after_expr(cx: &LateContext<'_>, local_id: HirId, after: &Expr<'_>) -> bool {
    let Some(block) = utils::get_enclosing_block(cx, local_id) else {
        return false;
    };

    // for _ in 1..3 {
    //    local
    // }
    //
    // let closure = || local;
    // closure();
    // closure();
    let loop_start = get_enclosing_loop_or_multi_call_closure(cx, after).map(|e| e.hir_id);

    let mut past_expr = false;
    for_each_expr(cx, block, |e| {
        if past_expr {
            if e.res_local_id() == Some(local_id) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(Descend::Yes)
            }
        } else if e.hir_id == after.hir_id {
            past_expr = true;
            ControlFlow::Continue(Descend::No)
        } else {
            past_expr = Some(e.hir_id) == loop_start;
            ControlFlow::Continue(Descend::Yes)
        }
    })
    .is_some()
}

struct VariableNameUsageVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    expr_id: HirId,
    names: FxHashSet<Symbol>,
    past_expr: bool,
}

impl<'tcx> Visitor<'tcx> for VariableNameUsageVisitor<'_, 'tcx> {
    type Result = ControlFlow<()>;

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) -> Self::Result {
        if self.past_expr
            && let Res::Local(_) = path.res
            && let [segment] = path.segments
            && self.names.contains(&segment.ident.name)
        {
            return ControlFlow::Break(());
        }

        walk_path(self, path)
    }

    fn visit_block(&mut self, block: &'tcx Block<'tcx>) -> Self::Result {
        if self.past_expr {
            let before = self.names.clone();
            walk_block(self, block)?;
            self.names = before;
            return ControlFlow::Continue(());
        }
        walk_block(self, block)
    }

    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) -> Self::Result {
        if self.past_expr
            && let StmtKind::Let(let_stmt) = stmt.kind
        {
            if let Some(init) = let_stmt.init {
                self.visit_expr(init)?;
            }

            let_stmt.pat.each_binding(|_, _, _, ident| {
                self.names.remove(&ident.name);
            });
        }
        walk_stmt(self, stmt)
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        if self.past_expr {
            return match expr.kind {
                ExprKind::If(
                    Expr {
                        kind: ExprKind::Let(let_expr),
                        ..
                    },
                    then,
                    else_,
                ) => {
                    self.visit_expr(let_expr.init)?;
                    let before = self.names.clone();
                    let_expr.pat.each_binding(|_, _, _, ident| {
                        self.names.remove(&ident.name);
                    });

                    self.visit_expr(then)?;
                    self.names = before;
                    if let Some(else_) = else_ {
                        self.visit_expr(else_)?;
                    }
                    ControlFlow::Continue(())
                },
                ExprKind::Closure(closure) => {
                    let body = self.cx.tcx.hir_body(closure.body);
                    let before = self.names.clone();
                    for param in body.params {
                        param.pat.each_binding(|_, _, _, ident| {
                            self.names.remove(&ident.name);
                        });
                    }
                    self.visit_expr(body.value)?;
                    self.names = before;
                    ControlFlow::Continue(())
                },
                ExprKind::Match(expr, arms, _) => {
                    self.visit_expr(expr)?;
                    for arm in arms {
                        let before = self.names.clone();
                        arm.pat.each_binding(|_, _, _, ident| {
                            self.names.remove(&ident.name);
                        });
                        if let Some(guard) = arm.guard {
                            self.visit_expr(guard)?;
                        }
                        self.visit_expr(arm.body)?;
                        self.names = before;
                    }
                    ControlFlow::Continue(())
                },
                _ => walk_expr(self, expr),
            };
        }

        self.past_expr = expr.hir_id == self.expr_id;
        if !self.past_expr {
            return walk_expr(self, expr);
        }

        ControlFlow::Continue(())
    }
}

/// Checks if any of the given variable names are used after the given expression. This can be
/// helpful to check if removing a block would cause shadowing of variables declared outside the
/// block.
#[expect(
    clippy::implicit_hasher,
    reason = "`FxHashSet` is preferred for rustc data structures"
)]
pub fn variable_names_used_after_expr(cx: &LateContext<'_>, names: FxHashSet<Symbol>, after: &Expr<'_>) -> bool {
    let Some(block) = utils::get_enclosing_block(cx, after.hir_id) else {
        return false;
    };

    let loop_start = get_enclosing_loop_or_multi_call_closure(cx, after).map(|e| e.hir_id);

    let mut visitor = VariableNameUsageVisitor {
        cx,
        expr_id: loop_start.unwrap_or(after.hir_id),
        names,
        past_expr: false,
    };
    visitor.visit_block(block).is_break() || !visitor.past_expr
}

/// Returns the set of variable names declared in the given pattern.
pub fn variable_names_of_pat(pat: &Pat<'_>) -> FxHashSet<Symbol> {
    let mut names = FxHashSet::default();
    pat.each_binding(|_, _, _, ident| {
        names.insert(ident.name);
    });
    names
}

/// Returns the set of variable names declared in the given block.
pub fn variable_names_of_block(block: &Block<'_>) -> FxHashSet<Symbol> {
    let mut names = FxHashSet::default();
    for stmt in block.stmts {
        if let StmtKind::Let(let_stmt) = stmt.kind {
            let_stmt.pat.each_binding(|_, _, _, ident| {
                names.insert(ident.name);
            });
        }
    }
    names
}
