use rustc::hir::*;
use rustc::hir::intravisit::{Visitor, NestedVisitorMap, walk_expr};
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use utils::{SpanlessEq, snippet, span_lint_and_then};

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
    map_expr: &'a Expr,
    key_expr: &'a Expr,
    witnessed: bool,
    removed: bool,
}

impl<'a, 'tcx> Visitor<'tcx> for MapRemoveVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        let eq = SpanlessEq::new(self.cx);
        if let ExprCall(ref fun, ref args) = expr.node {
            for arg in args {
                if eq.eq_expr(self.map_expr, arg) { 
                    self.witnessed = true;
                    return; 
                }
            }
        }
        if let ExprMethodCall(ref method, _, ref args) = expr.node {
            if method.name == "remove" && args.len() == 2 {
                if eq.eq_expr(self.map_expr, &args[0]) && eq.eq_expr(self.key_expr, &args[1]) {
                    self.removed = true;
                    return;
                }
            } else {
                for arg in args {
                    if eq.eq_expr(self.map_expr, &arg) { 
                        self.witnessed = true;
                        return;
                    }
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
                    let map_expr = &args[0];
                    let key_expr = &args[1];
                    let mut visitor = MapRemoveVisitor {
                        cx: cx,
                        map_expr: &map_expr,
                        key_expr: &key_expr,
                        witnessed: false,
                        removed: false,
                    };
                    walk_expr(&mut visitor, then);
                    if visitor.removed && !visitor.witnessed {
                        span_lint_and_then(
                            cx,
                            MAP_CONTAINS_THEN_REMOVE,
                            expr.span,
                            &"it looks like you are unnecessarily checking whether a key is contained in a HashMap before removing the key".to_string(),
                            |db| {
                                db.span_suggestion(
                                    expr.span,
                                    "consider replacing the `if` expression with the following, and removing the call to `remove` from the `then` block",
                                    format!("if {}.remove({}).is_some() {{ ... }}", snippet(cx, map_expr.span, ".."), snippet(cx, key_expr.span, "..")),
                                );
                            }                
                        );
                    }
                    return;
                }
                return;
            }
        }
    }
}

