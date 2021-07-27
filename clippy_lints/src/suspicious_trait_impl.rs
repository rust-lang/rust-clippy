use clippy_utils::diagnostics::span_lint;
use clippy_utils::{binop_traits, trait_ref_of_method, BINOP_TRAITS, OP_ASSIGN_TRAITS};
use if_chain::if_chain;
use rustc_hir as hir;
use rustc_hir::intravisit::{walk_expr, NestedVisitorMap, Visitor};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::map::Map;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Lints for suspicious operations in impls of arithmetic operators, e.g.
    /// subtracting elements in an Add impl.
    ///
    /// ### Why is this bad?
    /// This is probably a typo or copy-and-paste error and not intended.
    ///
    /// ### Example
    /// ```ignore
    /// impl Add for Foo {
    ///     type Output = Foo;
    ///
    ///     fn add(self, other: Foo) -> Foo {
    ///         Foo(self.0 - other.0)
    ///     }
    /// }
    /// ```
    pub SUSPICIOUS_ARITHMETIC_IMPL,
    suspicious,
    "suspicious use of operators in impl of arithmetic trait"
}

declare_clippy_lint! {
    /// ### What it does
    /// Lints for suspicious operations in impls of OpAssign, e.g.
    /// subtracting elements in an AddAssign impl.
    ///
    /// ### Why is this bad?
    /// This is probably a typo or copy-and-paste error and not intended.
    ///
    /// ### Example
    /// ```ignore
    /// impl AddAssign for Foo {
    ///     fn add_assign(&mut self, other: Foo) {
    ///         *self = *self - other;
    ///     }
    /// }
    /// ```
    pub SUSPICIOUS_OP_ASSIGN_IMPL,
    suspicious,
    "suspicious use of operators in impl of OpAssign trait"
}

declare_lint_pass!(SuspiciousImpl => [SUSPICIOUS_ARITHMETIC_IMPL, SUSPICIOUS_OP_ASSIGN_IMPL]);

impl<'tcx> LateLintPass<'tcx> for SuspiciousImpl {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        if_chain! {
            if let hir::ExprKind::Binary(binop, _, _) | hir::ExprKind::AssignOp(binop, ..) = expr.kind;
            if let Some((binop_trait_lang, op_assign_trait_lang)) = binop_traits(binop.node);
            if let Ok(binop_trait_id) = cx.tcx.lang_items().require(binop_trait_lang);
            if let Ok(op_assign_trait_id) = cx.tcx.lang_items().require(op_assign_trait_lang);

            // Check for more than one binary operation in the implemented function
            // Linting when multiple operations are involved can result in false positives
            let parent_fn = cx.tcx.hir().get_parent_item(expr.hir_id);
            if let hir::Node::ImplItem(impl_item) = cx.tcx.hir().get(parent_fn);
            if let hir::ImplItemKind::Fn(_, body_id) = impl_item.kind;
            let body = cx.tcx.hir().body(body_id);
            let parent_fn = cx.tcx.hir().get_parent_item(expr.hir_id);
            if let Some(trait_ref) = trait_ref_of_method(cx, parent_fn);
            let trait_id = trait_ref.path.res.def_id();
            if ![binop_trait_id, op_assign_trait_id].contains(&trait_id);
            if let Some(&(_, lint)) = [
                (&BINOP_TRAITS, SUSPICIOUS_ARITHMETIC_IMPL),
                (&OP_ASSIGN_TRAITS, SUSPICIOUS_OP_ASSIGN_IMPL),
            ]
                .iter()
                .find(|&(ts, _)| ts.iter().any(|&t| Ok(trait_id) == cx.tcx.lang_items().require(t)));
            if count_binops(&body.value) == 1;
            then {
                span_lint(
                    cx,
                    lint,
                    binop.span,
                    &format!("suspicious use of `{}` in `{}` impl", binop.node.as_str(), cx.tcx.item_name(trait_id)),
                );
            }
        }
    }
}

fn count_binops(expr: &hir::Expr<'_>) -> u32 {
    let mut visitor = BinaryExprVisitor::default();
    visitor.visit_expr(expr);
    visitor.nb_binops
}

#[derive(Default)]
struct BinaryExprVisitor {
    nb_binops: u32,
}

impl<'tcx> Visitor<'tcx> for BinaryExprVisitor {
    type Map = Map<'tcx>;

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'_>) {
        match expr.kind {
            hir::ExprKind::Binary(..)
            | hir::ExprKind::Unary(hir::UnOp::Not | hir::UnOp::Neg, _)
            | hir::ExprKind::AssignOp(..) => self.nb_binops += 1,
            _ => {},
        }

        walk_expr(self, expr);
    }

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::None
    }
}
