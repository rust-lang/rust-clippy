use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{higher, peel_hir_expr_while, SpanlessEq};
use rustc_hir::intravisit::Visitor;
use rustc_hir::{Block, Expr, ExprKind, Stmt, StmtKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// Checks for usage of `contains` to see if a value is not
    /// present on `HashSet` followed by a `insert`.
    ///
    /// ### Why is this bad?
    /// Using just `insert` and checking the returned `bool` is more efficient.
    ///
    /// ### Example
    /// ```rust
    /// use std::collections::HashSet;
    /// let mut set = HashSet::new();
    /// let value = 5;
    /// if !set.contains(&value) {
    ///     set.insert(value);
    ///     println!("inserted {value:?}");
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::collections::HashSet;
    /// let mut set = HashSet::new();
    /// let value = 5;
    /// if set.insert(&value) {
    ///     println!("inserted {value:?}");
    /// }
    /// ```
    #[clippy::version = "1.73.0"]
    pub HASHSET_INSERT_AFTER_CONTAINS,
    perf,
    "use of `contains` to see if a value is not present on a `HashSet` followed by a `insert`"
}
declare_lint_pass!(HashsetInsertAfterContains => [HASHSET_INSERT_AFTER_CONTAINS]);

impl<'tcx> LateLintPass<'tcx> for HashsetInsertAfterContains {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        let Some(higher::If {
            cond: cond_expr,
            then: then_expr,
            ..
        }) = higher::If::hir(expr)
        else {
            return;
        };

        let Some(contains_expr) = try_parse_contains(cx, cond_expr) else {
            return;
        };

        if !find_insert_calls(cx, &contains_expr, then_expr) {
            return;
        };
        span_lint(
            cx,
            HASHSET_INSERT_AFTER_CONTAINS,
            expr.span,
            "usage of `HashSet::insert` after `HashSet::contains`. Remove the usage of `HashSet::contains` and just call `HashSet::contains` instead",
        );
    }
}

struct ContainsExpr<'tcx> {
    receiver: &'tcx Expr<'tcx>,
    value: &'tcx Expr<'tcx>,
}
fn try_parse_contains<'tcx>(cx: &LateContext<'_>, expr: &'tcx Expr<'_>) -> Option<ContainsExpr<'tcx>> {
    let expr = peel_hir_expr_while(expr, |e| match e.kind {
        ExprKind::Unary(UnOp::Not, e) => Some(e),
        _ => None,
    });
    match expr.kind {
        ExprKind::MethodCall(
            path,
            receiver,
            [
                Expr {
                    kind: ExprKind::AddrOf(_, _, value),
                    span: value_span,
                    ..
                },
            ],
            _,
        ) => {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            if value_span.ctxt() == expr.span.ctxt()
                && is_type_diagnostic_item(cx, receiver_ty, sym::HashSet)
                && path.ident.name == sym!(contains)
            {
                Some(ContainsExpr { receiver, value })
            } else {
                None
            }
        },
        _ => None,
    }
}

struct InsertExpr<'tcx> {
    receiver: &'tcx Expr<'tcx>,
    value: &'tcx Expr<'tcx>,
}
fn try_parse_insert<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> Option<InsertExpr<'tcx>> {
    if let ExprKind::MethodCall(path, receiver, [value], _) = expr.kind {
        let receiver_ty = cx.typeck_results().expr_ty(receiver);
        if is_type_diagnostic_item(cx, receiver_ty, sym::HashSet) && path.ident.name == sym!(insert) {
            Some(InsertExpr { receiver, value })
        } else {
            None
        }
    } else {
        None
    }
}

fn find_insert_calls<'tcx>(cx: &LateContext<'tcx>, contains_expr: &ContainsExpr<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    let mut s = InsertSearcher {
        cx,
        receiver: contains_expr.receiver,
        value: contains_expr.value,
        should_lint: false,
    };
    s.visit_expr(expr);
    s.should_lint
}

struct InsertSearcher<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    /// The receiver expression used in the contains call.
    receiver: &'tcx Expr<'tcx>,
    /// The value expression used in the contains call.
    value: &'tcx Expr<'tcx>,
    /// Whether or a lint shoud be emitted.
    should_lint: bool,
}

impl<'tcx> Visitor<'tcx> for InsertSearcher<'_, 'tcx> {
    fn visit_block(&mut self, block: &'tcx Block<'_>) {
        for stmt in block.stmts {
            self.visit_stmt(stmt);
        }
        if let Some(expr) = block.expr {
            self.visit_expr(expr);
        }
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) {
        match try_parse_insert(self.cx, expr) {
            Some(insert_expr) => {
                if SpanlessEq::new(self.cx).eq_expr(self.receiver, insert_expr.receiver)
                    && SpanlessEq::new(self.cx).eq_expr(self.value, insert_expr.value)
                {
                    self.should_lint = true;
                }
            },
            _ => {
                if let ExprKind::Block(block, _) = expr.kind {
                    self.visit_block(block);
                }
            },
        }
    }

    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'_>) {
        match stmt.kind {
            StmtKind::Semi(e) => {
                self.visit_expr(e);
            },
            StmtKind::Expr(e) => self.visit_expr(e),
            _ => (),
        }
    }
}
