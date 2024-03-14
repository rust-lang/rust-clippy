use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::eq_expr_value;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::for_each_expr;
use rustc_ast::{BorrowKind, LitKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Local, Node, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of `seq.is_empty()` in an if-conditional where `seq` is a slice, array, or Vec,
    /// and in which the first element of the sequence is indexed.
    ///
    /// ### Why is this bad?
    /// This code is unnecessarily complicated and can instead be simplified to the use of an
    /// if..let expression which accessed the first element of the sequence.
    ///
    /// ### Example
    /// ```no_run
    /// let a: &[i32] = &[1];
    /// if !a.is_empty() {
    ///    let b = a[0];
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a: &[i32] = &[1];
    /// if let Some(b) = a.first() {
    ///
    /// }
    /// ```
    #[clippy::version = "1.78.0"]
    pub UNNECESSARY_INDEXING,
    complexity,
    "unnecessary use of `seq.is_empty()` in a conditional when if..let is more appropriate"
}

declare_lint_pass!(UnnecessaryIndexing => [UNNECESSARY_INDEXING]);

impl LateLintPass<'_> for UnnecessaryIndexing {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ rustc_hir::Expr<'_>) {
        if let Some(if_expr) = clippy_utils::higher::If::hir(expr)
            // check for negation
            && let ExprKind::Unary(UnOp::Not, unary_inner) = if_expr.cond.kind
            // check for call of is_empty
            && let ExprKind::MethodCall(method, conditional_receiver, _, _) = unary_inner.kind
            && method.ident.as_str() == "is_empty"
            && let expr_ty = cx.typeck_results().expr_ty(conditional_receiver).peel_refs()
            && (expr_ty.is_array_slice() || expr_ty.is_array() || is_type_diagnostic_item(cx, expr_ty, sym::Vec))
            && let ExprKind::Block(block, _) = if_expr.then.kind
        {
            // the receiver for the index operation
            let mut index_receiver: Option<&Expr<'_>> = None;
            // first local in the block - used as pattern for `Some(pat)`
            let mut first_local: Option<&Local<'_>> = None;
            // any other locals to be aware of, these are set to the value of `pat`
            let mut extra_locals: Vec<&Local<'_>> = vec![];
            // any other index expressions to replace with `pat` (or "element" if no local exists)
            let mut extra_exprs: Vec<&Expr<'_>> = vec![];

            for_each_expr(block.stmts, |x| {
                if let ExprKind::Index(receiver, index, _) = x.kind
                    && let ExprKind::Lit(lit) = index.kind
                    && let LitKind::Int(val, _) = lit.node
                    && eq_expr_value(cx, receiver, conditional_receiver)
                    && val.0 == 0
                {
                    index_receiver = Some(receiver);
                    if let Node::Local(local) = cx.tcx.parent_hir_node(x.hir_id) {
                        if first_local.is_none() {
                            first_local = Some(local);
                        } else {
                            extra_locals.push(local);
                        };
                    } else {
                        extra_exprs.push(x);
                    };
                } else if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, val) = x.kind
                    && eq_expr_value(cx, conditional_receiver, val)
                {
                    return ControlFlow::Break(());
                };

                ControlFlow::Continue::<()>(())
            });

            if let Some(receiver) = index_receiver {
                span_lint_and_then(
                    cx,
                    UNNECESSARY_INDEXING,
                    expr.span,
                    "condition can be simplified with if..let syntax",
                    |x| {
                        if let Some(first_local) = first_local {
                            x.span_suggestion(
                                if_expr.cond.span,
                                "consider using if..let syntax (variable may need to be dereferenced)",
                                format!(
                                    "let Some({}) = {}.first()",
                                    snippet(cx, first_local.pat.span, ".."),
                                    snippet(cx, receiver.span, "..")
                                ),
                                Applicability::Unspecified,
                            );
                            x.span_suggestion(first_local.span, "remove this line", "", Applicability::Unspecified);
                            if !extra_locals.is_empty() {
                                let extra_local_suggestions = extra_locals
                                    .iter()
                                    .map(|x| {
                                        (
                                            x.init.unwrap().span,
                                            snippet(cx, first_local.pat.span, "..").to_string(),
                                        )
                                    })
                                    .collect::<Vec<_>>();

                                x.multipart_suggestion(
                                    "initialize this variable to be the `Some` variant (may need dereferencing)",
                                    extra_local_suggestions,
                                    Applicability::Unspecified,
                                );
                            }
                            if !extra_exprs.is_empty() {
                                let index_accesses = extra_exprs
                                    .iter()
                                    .map(|x| (x.span, snippet(cx, first_local.pat.span, "..").to_string()))
                                    .collect::<Vec<_>>();

                                x.multipart_suggestion(
                                    "set this index to be the `Some` variant (may need dereferencing)",
                                    index_accesses,
                                    Applicability::Unspecified,
                                );
                            }
                        } else {
                            let mut index_accesses = vec![(
                                if_expr.cond.span,
                                format!("let Some(element) = {}.first()", snippet(cx, receiver.span, "..")),
                            )];
                            index_accesses.extend(extra_exprs.iter().map(|x| (x.span, "element".to_owned())));

                            x.multipart_suggestion(
                                "consider using if..let syntax (variable may need to be dereferenced)",
                                index_accesses,
                                Applicability::Unspecified,
                            );
                        }
                    },
                );
            }
        }
    }
}
