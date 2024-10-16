use std::ops::ControlFlow;

use rustc_ast::visit::VisitorResult;
use rustc_ast_ir::try_visit;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Block, Expr, ExprKind};

pub enum ReturnType<'tcx> {
    /// An implicit return.
    ///
    /// This is an expression that evaluates directly to a value, like a literal or operation.
    Implicit(&'tcx Expr<'tcx>),
    /// An explicit return.
    ///
    /// This is the return expression of `return <expr>`.
    Explicit(&'tcx Expr<'tcx>),
    /// An explicit unit type return.
    ///
    /// This is the return expression `return`.
    UnitReturnExplicit(&'tcx Expr<'tcx>),
    /// A `()` implicit return.
    ///
    /// The expression is the `ExprKind::If` with no `else` block.
    ///
    /// ```no_run
    /// fn example() -> () {
    ///     if true {
    ///
    ///     } // no else!
    /// }
    /// ```
    MissingElseImplicit(&'tcx Expr<'tcx>),
    /// A diverging implict return.
    ///
    /// ```no_run
    /// fn example() -> u8 {
    ///     { todo!(); }
    /// }
    /// ```
    DivergingImplicit(&'tcx Block<'tcx>),
}

pub trait ReturnVisitor {
    type Result: VisitorResult = ();

    fn visit_return(&mut self, return_type: ReturnType<'_>) -> Self::Result;
}

struct ExplicitReturnDriver<V>(V);

impl<V: ReturnVisitor> Visitor<'_> for ExplicitReturnDriver<V> {
    type Result = V::Result;
    type NestedFilter = intravisit::nested_filter::None;

    fn visit_expr(&mut self, expr: &Expr<'_>) -> Self::Result {
        if let ExprKind::Ret(ret_val_expr) = expr.kind {
            if let Some(ret_val_expr) = ret_val_expr {
                try_visit!(self.0.visit_return(ReturnType::Explicit(ret_val_expr)));
            } else {
                try_visit!(self.0.visit_return(ReturnType::UnitReturnExplicit(expr)));
            }
        }

        intravisit::walk_expr(self, expr)
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
                visitor.visit_return(ReturnType::DivergingImplicit(block))
            }
        },
        ExprKind::If(_, true_block, else_block) => {
            try_visit!(visit_implicit_returns(visitor, true_block));
            if let Some(expr) = else_block {
                visit_implicit_returns(visitor, expr)
            } else {
                visitor.visit_return(ReturnType::MissingElseImplicit(expr))
            }
        },
        ExprKind::Match(_, arms, _) => {
            for arm in arms {
                try_visit!(visit_implicit_returns(visitor, arm.body));
            }

            cont()
        },

        _ => visitor.visit_return(ReturnType::Implicit(expr)),
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
