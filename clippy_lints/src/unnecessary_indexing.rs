use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::source::snippet;
use clippy_utils::visitors::for_each_expr;
use rustc_ast::{LitKind, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind, HirId, LetStmt, Node, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability};
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

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
    #[clippy::version = "1.86.0"]
    pub UNNECESSARY_INDEXING,
    complexity,
    "unnecessary use of `seq.is_empty()` in a conditional when if..let is more appropriate"
}

declare_lint_pass!(UnnecessaryIndexing => [UNNECESSARY_INDEXING]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryIndexing {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'_ Expr<'tcx>) {
        if let Some(if_expr) = clippy_utils::higher::If::hir(expr)
            && !if_expr.cond.span.from_expansion()
            // check for negation
            && let ExprKind::Unary(UnOp::Not, unary_inner) = if_expr.cond.kind
            // check for call of is_empty
            && let ExprKind::MethodCall(method, conditional_receiver, _, _) = unary_inner.kind
            && method.ident.as_str() == "is_empty"
            && let expr_ty = cx.typeck_results().expr_ty(conditional_receiver)
            && let peeled = expr_ty.peel_refs()
            && (peeled.is_slice() || peeled.is_array() || peeled.is_diag_item(cx, sym::Vec))
            && let ExprKind::Block(block, _) = if_expr.then.kind
            // do not lint if conditional receiver is mutable reference
            && expr_ty.ref_mutability() != Some(Mutability::Mut)
            && let Some(con_path) = conditional_receiver.res_local_id()
            && let Some(r) = process_indexing(cx, block, con_path)
        {
            span_lint_and_then(
                cx,
                UNNECESSARY_INDEXING,
                expr.span,
                "condition can be simplified with `if..let` syntax",
                |diag| {
                    let receiver = snippet(cx, r.index_receiver_span, "..");
                    let mut suggestions: Vec<(Span, String)> = vec![];
                    let mut message = "consider using `if..let` syntax instead of indexing".to_string();
                    if let Some(first_local) = r.first_local
                        && let Some(name) = first_local.pat.simple_ident().map(|ident| ident.name)
                    {
                        suggestions.push((
                            if_expr.cond.span,
                            format!(
                                "let Some({}{name}) = {receiver}.first()",
                                // if we arent borrowing anything then we can pass a reference here for correctness
                                if r.extra_exprs_borrow.is_empty() { "&" } else { "" },
                            ),
                        ));
                        suggestions.push((first_local.span, String::new()));

                        if !r.extra_exprs_borrow.is_empty() {
                            suggestions.extend(
                                r.extra_exprs_borrow
                                    .iter()
                                    .chain(r.extra_exprs_copy.iter())
                                    .map(|x| (x.span, name.to_string())),
                            );

                            message.push_str(", and replacing indexing expression(s) with the value in `Some` variant");
                        } else if !r.extra_exprs_copy.is_empty() {
                            suggestions.extend(r.extra_exprs_copy.iter().map(|x| (x.span, name.to_string())));
                        }
                    } else if r.extra_exprs_borrow.is_empty() {
                        suggestions.push((if_expr.cond.span, format!("let Some(&element) = {receiver}.first()")));
                        suggestions.extend(r.extra_exprs_copy.iter().map(|x| (x.span, "element".to_owned())));
                    } else {
                        suggestions.push((if_expr.cond.span, format!("let Some(element) = {receiver}.first()")));
                        suggestions.extend(
                            r.extra_exprs_borrow
                                .iter()
                                .chain(r.extra_exprs_copy.iter())
                                .map(|x| (x.span, "element".to_owned())),
                        );
                    }

                    diag.multipart_suggestion(message, suggestions, Applicability::MaybeIncorrect);
                },
            );
        }
    }
}

struct IndexCheckResult<'a> {
    // span of the receiver for the index operation, only Some in the event the indexing is via a direct primitive
    index_receiver_span: Span,
    // first local in the block - used as pattern for `Some(pat)`
    first_local: Option<&'a LetStmt<'a>>,
    // any other index expressions to replace with `pat` (or "element" if no local exists)
    extra_exprs_borrow: Vec<&'a Expr<'a>>,
    // copied extra index expressions: if we only have these and no borrows we can provide a correct suggestion of `let
    // Some(&a) = ...`
    extra_exprs_copy: Vec<&'a Expr<'a>>,
}

/// Checks the block for any indexing of the conditional receiver. Returns `None` if the block
/// contains code that invalidates the lint, e.g., the receiver is accessed via a mutable reference.
fn process_indexing<'a>(
    cx: &LateContext<'a>,
    block: &'a Block<'a>,
    conditional_receiver_hid: HirId,
) -> Option<IndexCheckResult<'a>> {
    let mut index_receiver_span: Option<Span> = None;
    let mut first_local: Option<&LetStmt<'_>> = None;
    let mut extra_exprs_borrow: Vec<&Expr<'_>> = vec![];
    let mut extra_exprs_copy: Vec<&Expr<'_>> = vec![];

    // if res == Some(()), then mutation occurred
    // & therefore we should not lint on this
    let res = for_each_expr(cx, block, |x| {
        let adjustments = cx.typeck_results().expr_adjustments(x);
        if let ExprKind::Index(receiver, index, _) = x.kind
            && let ExprKind::Lit(lit) = index.kind
            && let LitKind::Int(val, _) = lit.node
            && receiver.res_local_id() == Some(conditional_receiver_hid)
            && val.0 == 0
        {
            index_receiver_span = Some(receiver.span);
            if let Node::LetStmt(local) = cx.tcx.parent_hir_node(x.hir_id) {
                if first_local.is_none() {
                    first_local = Some(local);
                    return ControlFlow::Continue::<()>(());
                }
            }

            if let Node::Expr(x) = cx.tcx.parent_hir_node(x.hir_id)
                && let ExprKind::AddrOf(_, _, _) = x.kind
            {
                extra_exprs_borrow.push(x);
            } else {
                extra_exprs_copy.push(x);
            }
        } else if adjustments.iter().any(|adjustment| {
            matches!(
                adjustment.kind,
                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { .. }))
            )
        }) {
            // do not lint on mutable auto borrows (https://github.com/rust-lang/rust-clippy/pull/12464#discussion_r1600352696)
            return ControlFlow::Break(());
        } else if let ExprKind::AddrOf(_, Mutability::Mut, _) = x.kind {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue::<()>(())
    });

    res.is_none().then_some(IndexCheckResult {
        index_receiver_span: index_receiver_span?,
        first_local,
        extra_exprs_borrow,
        extra_exprs_copy,
    })
}
