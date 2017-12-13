use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::hir::*;
use rustc::hir::intravisit::{walk_path, walk_expr, walk_body, walk_impl_item, NestedVisitorMap, Visitor};
use utils::{in_macro, span_lint_and_then};
use syntax::ast::NodeId;
use syntax_pos::symbol::keywords::SelfType;
use syntax::ptr::P;

/// **What it does:** Checks for unnecessary repetition of structure name when a
/// replacement with `Self` is applicable.
///
/// **Why is this bad?** Unnecessary repetition. Mixed use of `Self` and struct
/// name
/// feels inconsistent.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// struct Foo {}
/// impl Foo {
///     fn new() -> Foo {
///         Foo {}
///     }
/// }
/// ```
/// could be
/// ```
/// struct Foo {}
/// impl Foo {
///     fn new() -> Self {
///         Self {}
///     }
/// }
/// ```
declare_lint! {
    pub USE_SELF,
    Allow,
    "Unnecessary structure name repetition whereas `Self` is applicable"
}

#[derive(Copy, Clone, Default)]
pub struct UseSelf;

impl LintPass for UseSelf {
    fn get_lints(&self) -> LintArray {
        lint_array!(USE_SELF)
    }
}

const SEGMENTS_MSG: &str = "segments should be composed of at least 1 element";

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UseSelf {
    fn check_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx Item) {
        if in_macro(item.span) {
            return;
        }
        if_chain! {
            if let ItemImpl(.., ref item_type, ref refs) = item.node;
            if let Ty_::TyPath(QPath::Resolved(_, ref self_path)) = item_type.node;
            then {
                let parameters = &self_path.segments.last().expect(SEGMENTS_MSG).parameters;
                    let visitor = &mut UseSelfVisitor {
                        self_path,
                        cx,
                        body_id: None,
                        self_type: item_type,
                    };
                    for impl_item_ref in refs {
                        visitor.visit_impl_item(cx.tcx.hir.impl_item(impl_item_ref.id));
                    }
            }
        }
    }
}

struct UseSelfVisitor<'a, 'tcx: 'a> {
    self_type: &'a P<Ty>,
    self_path: &'a Path,
    cx: &'a LateContext<'a, 'tcx>,
    body_id: Option<BodyId>,
}

impl<'a, 'tcx> Visitor<'tcx> for UseSelfVisitor<'a, 'tcx> {
    // fn visit_path(&mut self, path: &'tcx Path, _id: NodeId) {
    //     if self.self_path.def == path.def && path.segments.last().expect(SEGMENTS_MSG).name != SelfType.name() {
    //         span_lint_and_then(self.cx, USE_SELF, path.span, "unnecessary structure name repetition", |db| {
    //             db.span_suggestion(path.span, "use the applicable keyword", "Self".to_owned());
    //         });
    //     }

    //     walk_path(self, path);
    // }

    // fn visit_impl_item(&mut self, impl_item: &'tcx ImplItem) {
    //     println!("{:?}", impl_item);
    //     walk_impl_item(self, impl_item);
    // }

    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.cx.tcx.hir)
    }

    //TODO: This is just a hack to see if having the body id is useful
    fn visit_body(&mut self, body: &'tcx Body) {
        self.body_id = Some(body.id());
        walk_body(self, body);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr) {
        if let Expr_::ExprStruct(ref path, _, _) = expr.node {
            let segment = match *path {
                QPath::Resolved(_, ref path) => path.segments
                                                    .last()
                                                    .expect(SEGMENTS_MSG),
                QPath::TypeRelative(_, ref segment) => segment,
            };
            
            if let Some(body_id) = self.body_id {
                if segment.name != SelfType.name() {
                    let ty = self.cx.tcx.body_tables(body_id).expr_ty(expr);
                    println!("Self `P<Ty>`: {:?}", self.self_type);
                    println!("Self `Path`: {:?}", self.self_path);
                    println!("Struct Literal `TyS`: {:?}", ty);
                    println!();
                }
            }
        }

        walk_expr(self, expr);
    }
}
