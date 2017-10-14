use std::borrow::Cow;
use rustc::hir::*;
use rustc::hir::intravisit::{walk_expr, NestedVisitorMap, Visitor};
use rustc::lint::LateContext;
use syntax::ast::Name;
use syntax::codemap::Span;
use utils::{get_pat_name, is_adjusted, match_var, snippet, walk_ptrs_expr};

/// a `Vec` of `Span`s
pub type Spans = Vec<Span>;

/// a `Vec` of `Span`s with replacements
pub type SpansAndRepls = Vec<(Span, Cow<'static, str>)>;

/// for each `.clone()`, etc. call, get the list of spans of method calls that
/// need to be changed with suggestions and the list of spans of method calls
/// whose methods need to be changed, too
pub fn get_spans(
    cx: &LateContext,
    opt_body_id: Option<BodyId>,
    idx: usize,
    replacements: &'static [(&'static str, &'static str)],
) -> Option<(SpansAndRepls, Spans)> {
    if let Some(body) = opt_body_id.map(|id| cx.tcx.hir.body(id)) {
        get_binding_name(&body.arguments[idx])
            .map_or_else(|| Some((vec![], vec![])),
                         |name| extract_clone_suggestions(cx, name, replacements, body))
    } else {
        Some((vec![], vec![]))
    }
}

fn extract_clone_suggestions<'a, 'tcx: 'a>(
    cx: &LateContext<'a, 'tcx>,
    name: Name,
    replace: &'static [(&'static str, &'static str)],
    body: &'tcx Body,
) -> Option<(SpansAndRepls, Spans)> {
    let mut visitor = PtrCloneVisitor {
        cx,
        name,
        replace,
        spans: vec![],
        ref_calls: vec![],
        abort: false,
    };
    visitor.visit_body(body);
    if visitor.abort {
        None
    } else {
        Some((visitor.spans, visitor.ref_calls))
    }
}

struct PtrCloneVisitor<'a, 'tcx: 'a> {
    cx: &'a LateContext<'a, 'tcx>,
    name: Name,
    replace: &'static [(&'static str, &'static str)],
    spans: Vec<(Span, Cow<'static, str>)>,
    ref_calls: Vec<Span>,
    abort: bool,
}

fn check_args(v: &PtrCloneVisitor, args: &[Expr]) -> bool {
    for arg in args {
        let deref_arg = walk_ptrs_expr(arg);
        if match_var(deref_arg, v.name) && !is_adjusted(v.cx, deref_arg) {
            return true;
        }
    }
    false
}

impl<'a, 'tcx: 'a> Visitor<'tcx> for PtrCloneVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr) {
        if self.abort {
            return;
        }
        match expr.node {
            ExprMethodCall(ref seg, _, ref args) => {
                if args.len() == 1 && match_var(&args[0], self.name) {
                    if seg.name == "capacity" {
                        self.abort = true;
                        return;
                    }
                    for &(fn_name, suffix) in self.replace {
                        if seg.name == fn_name {
                            self.spans
                                .push((expr.span, snippet(self.cx, args[0].span, "_") + suffix));
                            return;
                        }
                    }
                    return;
                } else if check_args(self, args) {
                    self.ref_calls.push(expr.span);
                }
            }
            ExprCall(_, ref args) => {
                if check_args(self, args) {
                    self.ref_calls.push(expr.span)
                }
            }
            _ => ()
        }
        walk_expr(self, expr);
    }

    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }
}

fn get_binding_name(arg: &Arg) -> Option<Name> {
    get_pat_name(&arg.pat)
}
