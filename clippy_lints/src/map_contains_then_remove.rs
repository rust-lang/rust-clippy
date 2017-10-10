use rustc::hir::*;
use rustc::hir::def::Def;
use rustc::hir::intravisit::{Visitor, NestedVisitorMap, walk_expr};
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass, LintContext};
use syntax::ast::NodeId;
use utils::span_help_and_lint;

/// **What it does:** TODO
///
/// **Why is this bad?** TODO
///
/// **Known problems:** TODO
///
/// **Example:** TODO

declare_lint!{
    pub MAP_CONTAINS_THEN_REMOVE,
    Warn,
    "TODO:description"
}

struct MapRemoveVisitor<'a, 'tcx: 'a> {
    cx: &'a LateContext<'a, 'tcx>,
    map_id: NodeId,
    key_id: NodeId,
    removed: bool,
}

impl<'a, 'tcx> Visitor<'tcx> for MapRemoveVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        if let ExprMethodCall(ref method, _, ref args) = expr.node {
            if method.name == "remove" && args.len() == 2 {
                self.cx.sess().span_note_without_error(expr.span, "found a candidate, now checking if map/key IDs match");
                if same_var(self.cx, &args[0], self.map_id) && same_var(self.cx, &args[1], self.key_id) {
                    self.removed = true;
                }
                else {
                    println!("...no match");
                }
            }
        }
        walk_expr(self, expr);
    }

    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }
}

pub struct ContainsKey;

impl LintPass for ContainsKey {
    fn get_lints(&self) -> LintArray {
        lint_array!(MAP_CONTAINS_THEN_REMOVE)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for ContainsKey {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        if let ExprIf(ref check, ref then, _) = expr.node {
            if let ExprMethodCall(ref method, _, ref args) = check.node {
                if method.name == "contains_key" && args.len() == 2 {
                    let map_id = args[0].id;
                    let key_id = args[1].id;
                    let msg = &"";
                    let help = &"";
                    let mut visitor = MapRemoveVisitor {
                        cx: cx,
                        map_id: map_id,
                        key_id: key_id,
                        removed: false,
                    };
                    walk_expr(&mut visitor, then);
                    if visitor.removed {
                        span_help_and_lint(cx, MAP_CONTAINS_THEN_REMOVE, expr.span, msg, help);
                    }
                    return;
                }
            }
        }
        return;
    }
}

fn same_var<'a, 'tcx>(cx: &LateContext<'a, 'tcx>, expr: &Expr, var: NodeId) -> bool {
    if_let_chain! {[
        let ExprPath(ref qpath) = expr.node,
        let QPath::Resolved(None, ref path) = *qpath,
        path.segments.len() == 1,
        let Def::Local(local_id) = cx.tables.qpath_def(qpath, expr.hir_id),
        local_id == var,
    ], {
        return true;
    }}
    return false;
}
