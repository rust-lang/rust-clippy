use crate::utils::paths;
use crate::utils::{match_qpath, match_trait_method, span_lint};
use rustc::hir::def::Res;
use rustc::hir::intravisit::{walk_expr, NestedVisitorMap, Visitor};
use rustc::hir::{Body, Expr, ExprKind, HirId, Pat};
use rustc::lint::LateContext;
use rustc::middle::expr_use_visitor::*;
use rustc::middle::mem_categorization::{cmt_, Categorization};
use rustc::ty;
use syntax::source_map::Span;

use if_chain::if_chain;

use super::UNNECESSARY_FILTER_MAP;

fn can_be_ref(cx: &LateContext<'_, '_>, body: &Body, arg_id: HirId) -> bool {
    struct CanBeRefDelegate {
        arg_id: HirId,
        result: bool,
    }

    impl<'tcx> Delegate<'tcx> for CanBeRefDelegate {
        fn consume(&mut self, _: HirId, _: Span, cmt: &cmt_<'tcx>, consume_mode: ConsumeMode) {
            if_chain! {
                if self.result;
                if consume_mode != ConsumeMode::Copy;
                if let Categorization::Local(id) = cmt.cat;
                if self.arg_id == id;
                then {
                    self.result = false;
                }
            }
        }
        fn borrow(&mut self, _: HirId, _: Span, cmt: &cmt_<'tcx>, _: ty::Region<'_>, bk: ty::BorrowKind, _: LoanCause) {
            if_chain! {
                if self.result;
                if bk != ty::BorrowKind::ImmBorrow;
                if let Categorization::Local(id) = cmt.cat;
                if self.arg_id == id;
                then {
                    self.result = false;
                }
            }
        }
        fn mutate(&mut self, _: HirId, _: Span, cmt: &cmt_<'tcx>, mutate_mode: MutateMode) {
            if_chain! {
                if self.result;
                if mutate_mode != MutateMode::Init;
                if let Categorization::Local(id) = cmt.cat;
                if self.arg_id == id;
                then {
                    self.result = false;
                }
            }
        }

        fn matched_pat(&mut self, _: &Pat, _: &cmt_<'tcx>, _: MatchMode) {}
        fn consume_pat(&mut self, _: &Pat, _: &cmt_<'tcx>, _: ConsumeMode) {}
        fn decl_without_init(&mut self, _: HirId, _: Span) {}
    }

    let mut delegate = CanBeRefDelegate { arg_id, result: true };

    let closure_def_id = body.id().hir_id.owner_def_id();
    let region_scope_tree = &cx.tcx.region_scope_tree(closure_def_id);
    ExprUseVisitor::new(
        &mut delegate,
        cx.tcx,
        closure_def_id,
        cx.param_env,
        region_scope_tree,
        cx.tables,
        None,
    )
    .consume_body(body);

    delegate.result
}

pub(super) fn lint(cx: &LateContext<'_, '_>, expr: &Expr, args: &[Expr]) {
    if !match_trait_method(cx, expr, &paths::ITERATOR) {
        return;
    }

    if let ExprKind::Closure(_, _, body_id, ..) = args[1].node {
        let body = cx.tcx.hir().body(body_id);
        let arg_id = body.params[0].pat.hir_id;

        let (mut found_mapping, mut found_filtering) = check_expression(&cx, arg_id, &body.value);

        let mut return_visitor = ReturnVisitor::new(&cx, arg_id);
        return_visitor.visit_expr(&body.value);
        found_mapping |= return_visitor.found_mapping;
        found_filtering |= return_visitor.found_filtering;

        if !found_filtering {
            span_lint(
                cx,
                UNNECESSARY_FILTER_MAP,
                expr.span,
                "this `.filter_map` can be written more simply using `.map`",
            );
            return;
        }

        if !found_mapping && can_be_ref(cx, body, arg_id) {
            span_lint(
                cx,
                UNNECESSARY_FILTER_MAP,
                expr.span,
                "this `.filter_map` can be written more simply using `.filter`",
            );
            return;
        }
    }
}

// returns (found_mapping, found_filtering)
fn check_expression<'a, 'tcx>(cx: &'a LateContext<'a, 'tcx>, arg_id: HirId, expr: &'tcx Expr) -> (bool, bool) {
    match &expr.node {
        ExprKind::Call(ref func, ref args) => {
            if_chain! {
                if let ExprKind::Path(ref path) = func.node;
                then {
                    if match_qpath(path, &paths::OPTION_SOME) {
                        if_chain! {
                            if let ExprKind::Path(path) = &args[0].node;
                            if let Res::Local(ref local) = cx.tables.qpath_res(path, args[0].hir_id);
                            then {
                                if arg_id == *local {
                                    return (false, false)
                                }
                            }
                        }
                        return (true, false);
                    } else {
                        // We don't know. It might do anything.
                        return (true, true);
                    }
                }
            }
            (true, true)
        },
        ExprKind::Block(ref block, _) => {
            if let Some(expr) = &block.expr {
                check_expression(cx, arg_id, &expr)
            } else {
                (false, false)
            }
        },
        ExprKind::Match(_, ref arms, _) => {
            let mut found_mapping = false;
            let mut found_filtering = false;
            for arm in arms {
                let (m, f) = check_expression(cx, arg_id, &arm.body);
                found_mapping |= m;
                found_filtering |= f;
            }
            (found_mapping, found_filtering)
        },
        ExprKind::Path(path) if match_qpath(path, &paths::OPTION_NONE) => (false, true),
        _ => (true, true),
    }
}

struct ReturnVisitor<'a, 'tcx> {
    cx: &'a LateContext<'a, 'tcx>,
    arg_id: HirId,
    // Found a non-None return that isn't Some(input)
    found_mapping: bool,
    // Found a return that isn't Some
    found_filtering: bool,
}

impl<'a, 'tcx> ReturnVisitor<'a, 'tcx> {
    fn new(cx: &'a LateContext<'a, 'tcx>, arg_id: HirId) -> ReturnVisitor<'a, 'tcx> {
        ReturnVisitor {
            cx,
            arg_id,
            found_mapping: false,
            found_filtering: false,
        }
    }
}

impl<'a, 'tcx> Visitor<'tcx> for ReturnVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        if let ExprKind::Ret(Some(expr)) = &expr.node {
            let (found_mapping, found_filtering) = check_expression(self.cx, self.arg_id, expr);
            self.found_mapping |= found_mapping;
            self.found_filtering |= found_filtering;
        } else {
            walk_expr(self, expr);
        }
    }

    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }
}
