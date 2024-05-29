use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{eq_expr_value, path_to_local, path_to_local_id};
use rustc_ast::{BorrowKind, LitKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind, LetStmt, Node, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of `seq.is_empty()` in an if-conditional where `seq` is a slice, array, or Vec,
    /// and in which the first element of the sequence is indexed.
    ///
    /// ### Why is this bad?
    /// This code is unnecessarily complicated and can instead be simplified to the use of an
    /// if..let expression which accesses the first element of the sequence.
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
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        if let Some(if_expr) = clippy_utils::higher::If::hir(expr)
            // check for negation
            && let ExprKind::Unary(UnOp::Not, unary_inner) = if_expr.cond.kind
            // check for call of is_empty
            && let ExprKind::MethodCall(method, conditional_receiver, _, _) = unary_inner.kind
            && method.ident.as_str() == "is_empty"
            && let typeck_results = cx.typeck_results()
            // do not lint on mutable auto borrows (https://github.com/rust-lang/rust-clippy/pull/12464#discussion_r1600352696)
            && let adjustments = typeck_results.expr_adjustments(conditional_receiver)
            && !adjustments.iter().any(|adjustment| {
                matches!(adjustment.kind, Adjust::Borrow(AutoBorrow::Ref(_, AutoBorrowMutability::Mut {
                    allow_two_phase_borrow: _
                })))
            })
            // do not lint if receiver is a mutable reference
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, _) = conditional_receiver.kind
            && let expr_ty = typeck_results.expr_ty(conditional_receiver).peel_refs()
            && (expr_ty.is_slice() || expr_ty.is_array() || is_type_diagnostic_item(cx, expr_ty, sym::Vec))
            && let ExprKind::Block(block, _) = if_expr.then.kind
        {
            let result = process_indexing(cx, block, conditional_receiver);

            if let Some(r) = result
                && let Some(receiver) = r.index_receiver
            {
                span_lint_and_then(
                    cx,
                    UNNECESSARY_INDEXING,
                    expr.span,
                    "condition can be simplified with if..let syntax",
                    |diag| {
                        if let Some(first_local) = r.first_local {
                            diag.span_suggestion(
                                if_expr.cond.span,
                                "consider using if..let syntax (variable may need to be dereferenced)",
                                format!(
                                    "let Some({}) = {}.first()",
                                    snippet(cx, first_local.pat.span, ".."),
                                    snippet(cx, receiver.span, "..")
                                ),
                                Applicability::Unspecified,
                            );
                            diag.span_suggestion(first_local.span, "remove this line", "", Applicability::Unspecified);
                            if !r.extra_locals.is_empty() {
                                let extra_local_suggestions = r
                                    .extra_locals
                                    .iter()
                                    .map(|x| {
                                        (
                                            x.init.unwrap().span,
                                            snippet(cx, first_local.pat.span, "..").to_string(),
                                        )
                                    })
                                    .collect::<Vec<_>>();

                                diag.multipart_suggestion(
                                    "initialize this variable to be the `Some` variant (may need dereferencing)",
                                    extra_local_suggestions,
                                    Applicability::Unspecified,
                                );
                            }
                            if !r.extra_exprs.is_empty() {
                                let index_accesses = r
                                    .extra_exprs
                                    .iter()
                                    .map(|x| (x.span, snippet(cx, first_local.pat.span, "..").to_string()))
                                    .collect::<Vec<_>>();

                                diag.multipart_suggestion(
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
                            index_accesses.extend(r.extra_exprs.iter().map(|x| (x.span, "element".to_owned())));

                            diag.multipart_suggestion(
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

struct IndexCheckResult<'a> {
    // the receiver for the index operation
    pub index_receiver: Option<&'a Expr<'a>>,
    // first local in the block - used as pattern for `Some(pat)`
    pub first_local: Option<&'a LetStmt<'a>>,
    // any other locals to be aware of, these are set to the value of `pat`
    pub extra_locals: Vec<&'a LetStmt<'a>>,
    // any other index expressions to replace with `pat` (or "element" if no local exists)
    pub extra_exprs: Vec<&'a Expr<'a>>,
}
impl<'a> IndexCheckResult<'a> {
    pub fn new() -> Self {
        IndexCheckResult {
            index_receiver: None,
            first_local: None,
            extra_locals: vec![],
            extra_exprs: vec![],
        }
    }
}

/// Checks the block for any indexing of the conditional receiver. Returns `None` if the block
/// contains code that invalidates the lint, e.g., the receiver is accessed via a mutable reference.
fn process_indexing<'a>(
    cx: &'a LateContext<'_>,
    block: &'a Block<'_>,
    conditional_receiver: &'a Expr<'_>,
) -> Option<IndexCheckResult<'a>> {
    let mut result = IndexCheckResult::new();

    let mut index_receiver: Option<&Expr<'_>> = None;
    let mut first_local: Option<&LetStmt<'_>> = None;
    let mut extra_locals: Vec<&LetStmt<'_>> = vec![];
    let mut extra_exprs: Vec<&Expr<'_>> = vec![];

    // if res == Some(()), then mutation occurred
    // & therefore we should not lint on this
    let res = for_each_expr(block.stmts, |x| {
        if let ExprKind::Index(receiver, index, _) = x.kind
            && let ExprKind::Lit(lit) = index.kind
            && let LitKind::Int(val, _) = lit.node
            && let Some(con_path) = path_to_local(conditional_receiver)
            && path_to_local_id(receiver, con_path)
            && val.0 == 0
        {
            index_receiver = Some(receiver);
            if let Node::LetStmt(local) = cx.tcx.parent_hir_node(x.hir_id) {
                if first_local.is_none() {
                    first_local = Some(local);
                } else {
                    extra_locals.push(local);
                };
            } else {
                extra_exprs.push(x);
            };
        } else if let ExprKind::AddrOf(_, Mutability::Mut, val) = x.kind
            && eq_expr_value(cx, conditional_receiver, val)
        {
            return ControlFlow::Break(());
        };

        ControlFlow::Continue::<()>(())
    });

    if res.is_none() {
        result.extra_exprs = extra_exprs;
        result.extra_locals = extra_locals;
        result.first_local = first_local;
        result.index_receiver = index_receiver;
        Some(result)
    } else {
        None
    }
}
