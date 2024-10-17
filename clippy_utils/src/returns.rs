use std::ops::ControlFlow;

use rustc_ast::visit::VisitorResult;
use rustc_ast_ir::try_visit;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Block, Expr, ExprKind};

pub trait ReturnVisitor {
    type Result: VisitorResult = ();

    fn visit_implicit_return(&mut self, expr: &Expr<'_>) -> Self::Result {
        self.visit_return(expr)
    }

    fn visit_explicit_return(&mut self, expr: &Expr<'_>) -> Self::Result {
        self.visit_return(expr)
    }

    /// In the example below, this function will be called with the `{ todo!(); }` block
    /// after the `;` due to the `NeverToAny` adjustment leading to the block returning `u8`
    /// with no expression directly attributable.
    /// ```no_run
    /// fn example() -> u8 {
    ///     { todo!(); }
    /// }
    /// ```
    fn visit_diverging_implicit_return(&mut self, block: &Block<'_>) -> Self::Result;
    fn visit_return(&mut self, expr: &Expr<'_>) -> Self::Result;
}

struct ExplicitReturnDriver<V>(V);

impl<V: ReturnVisitor> Visitor<'_> for ExplicitReturnDriver<V> {
    type Result = V::Result;
    type NestedFilter = intravisit::nested_filter::None;

    fn visit_expr(&mut self, expr: &Expr<'_>) -> Self::Result {
        if let ExprKind::Ret(Some(ret_val_expr)) = expr.kind {
            self.0.visit_explicit_return(ret_val_expr)
        } else {
            intravisit::walk_expr(self, expr)
        }
    }
}

fn visit_implicit_returns<V>(visitor: &mut V, expr: &Expr<'_>) -> V::Result
where
    V: ReturnVisitor,
{
    let cont = || V::Result::from_branch(ControlFlow::Continue(()));
    match expr.kind {
        ExprKind::Block(block, _) => {
            if let Some(expr) = block.expr {
                visit_implicit_returns(visitor, expr)
            } else {
                visitor.visit_diverging_implicit_return(block)
            }
        },
        ExprKind::If(_, true_block, else_block) => {
            try_visit!(visit_implicit_returns(visitor, true_block));
            visit_implicit_returns(visitor, else_block.unwrap())
        },
        ExprKind::Match(_, arms, _) => {
            for arm in arms {
                try_visit!(visit_implicit_returns(visitor, arm.body));
            }

            cont()
        },

        _ => visitor.visit_implicit_return(expr),
    }
}

pub fn visit_returns<V>(visitor: V, expr: &Expr<'_>) -> V::Result
where
    V: ReturnVisitor,
{
    let mut explicit_driver = ExplicitReturnDriver(visitor);
    try_visit!(explicit_driver.visit_expr(expr));

    visit_implicit_returns(&mut explicit_driver.0, expr)
}
