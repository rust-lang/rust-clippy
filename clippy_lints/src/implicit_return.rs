use clippy_utils::{
    diagnostics::span_lint_and_sugg,
    get_async_fn_body, is_async_fn,
    source::{snippet_with_applicability, snippet_with_context, walk_span_to_context},
    visitors::visit_break_exprs,
};
use rustc_errors::Applicability;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Block, Body, Expr, ExprKind, FnDecl, FnRetTy, HirId};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for missing return statements at the end of a block.
    ///
    /// ### Why is this bad?
    /// Actually omitting the return keyword is idiomatic Rust code. Programmers
    /// coming from other languages might prefer the expressiveness of `return`. It's possible to miss
    /// the last returning statement because the only difference is a missing `;`. Especially in bigger
    /// code with multiple return paths having a `return` keyword makes it easier to find the
    /// corresponding statements.
    ///
    /// ### Example
    /// ```rust
    /// fn foo(x: usize) -> usize {
    ///     x
    /// }
    /// ```
    /// add return
    /// ```rust
    /// fn foo(x: usize) -> usize {
    ///     return x;
    /// }
    /// ```
    pub IMPLICIT_RETURN,
    restriction,
    "use a return statement like `return expr` instead of an expression"
}

declare_lint_pass!(ImplicitReturn => [IMPLICIT_RETURN]);

fn lint_return(cx: &LateContext<'_>, span: Span) {
    let mut app = Applicability::MachineApplicable;
    let snip = snippet_with_applicability(cx, span, "..", &mut app);
    span_lint_and_sugg(
        cx,
        IMPLICIT_RETURN,
        span,
        "missing `return` statement",
        "add `return` as shown",
        format!("return {}", snip),
        app,
    );
}

fn lint_break(cx: &LateContext<'_>, break_span: Span, expr_span: Span) {
    let mut app = Applicability::MachineApplicable;
    let snip = snippet_with_context(cx, expr_span, break_span.ctxt(), "..", &mut app).0;
    span_lint_and_sugg(
        cx,
        IMPLICIT_RETURN,
        break_span,
        "missing `return` statement",
        "change `break` to `return` as shown",
        format!("return {}", snip),
        app,
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LintLocation {
    /// The lint was applied to a parent expression.
    Parent,
    /// The lint was applied to this expression, a child, or not applied.
    Inner,
}
impl LintLocation {
    fn still_parent(self, b: bool) -> Self {
        if b { self } else { Self::Inner }
    }

    fn is_parent(self) -> bool {
        self == Self::Parent
    }
}

// Gets the call site if the span is in a child context. Otherwise returns `None`.
fn get_call_site(span: Span, ctxt: SyntaxContext) -> Option<Span> {
    (span.ctxt() != ctxt).then(|| walk_span_to_context(span, ctxt).unwrap_or(span))
}

fn lint_implicit_returns(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    // The context of the function body.
    ctxt: SyntaxContext,
    // Whether the expression is from a macro expansion.
    call_site_span: Option<Span>,
) -> LintLocation {
    match expr.kind {
        ExprKind::Block(
            Block {
                expr: Some(block_expr), ..
            },
            _,
        ) => lint_implicit_returns(
            cx,
            block_expr,
            ctxt,
            call_site_span.or_else(|| get_call_site(block_expr.span, ctxt)),
        )
        .still_parent(call_site_span.is_some()),

        ExprKind::If(_, then_expr, Some(else_expr)) => {
            // Both `then_expr` or `else_expr` are required to be blocks in the same context as the `if`. Don't
            // bother checking.
            let res = lint_implicit_returns(cx, then_expr, ctxt, call_site_span).still_parent(call_site_span.is_some());
            if res.is_parent() {
                // The return was added as a parent of this if expression.
                return res;
            }
            lint_implicit_returns(cx, else_expr, ctxt, call_site_span).still_parent(call_site_span.is_some())
        },

        ExprKind::Match(_, arms, _) => {
            for arm in arms {
                let res = lint_implicit_returns(
                    cx,
                    arm.body,
                    ctxt,
                    call_site_span.or_else(|| get_call_site(arm.body.span, ctxt)),
                )
                .still_parent(call_site_span.is_some());
                if res.is_parent() {
                    // The return was added as a parent of this match expression.
                    return res;
                }
            }
            LintLocation::Inner
        },

        ExprKind::Loop(block, ..) => {
            let mut add_return = false;
            visit_break_exprs(block, |break_expr, dest, sub_expr| {
                if dest.target_id.ok() == Some(expr.hir_id) {
                    if call_site_span.is_none() && break_expr.span.ctxt() == ctxt {
                        // At this point sub_expr can be `None` in async functions which either diverge, or return the
                        // unit type.
                        if let Some(sub_expr) = sub_expr {
                            lint_break(cx, break_expr.span, sub_expr.span);
                        }
                    } else {
                        // the break expression is from a macro call, add a return to the loop
                        add_return = true;
                    }
                }
            });
            if add_return {
                #[allow(clippy::option_if_let_else)]
                if let Some(span) = call_site_span {
                    lint_return(cx, span);
                    LintLocation::Parent
                } else {
                    lint_return(cx, expr.span);
                    LintLocation::Inner
                }
            } else {
                LintLocation::Inner
            }
        },

        // If expressions without an else clause, and blocks without a final expression can only be the final expression
        // if they are divergent, or return the unit type.
        ExprKind::If(_, _, None) | ExprKind::Block(Block { expr: None, .. }, _) | ExprKind::Ret(_) => {
            LintLocation::Inner
        },

        // Any divergent expression doesn't need a return statement.
        ExprKind::MethodCall(..)
        | ExprKind::Call(..)
        | ExprKind::Binary(..)
        | ExprKind::Unary(..)
        | ExprKind::Index(..)
            if cx.typeck_results().expr_ty(expr).is_never() =>
        {
            LintLocation::Inner
        },

        _ =>
        {
            #[allow(clippy::option_if_let_else)]
            if let Some(span) = call_site_span {
                lint_return(cx, span);
                LintLocation::Parent
            } else {
                lint_return(cx, expr.span);
                LintLocation::Inner
            }
        },
    }
}

impl<'tcx> LateLintPass<'tcx> for ImplicitReturn {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        span: Span,
        _: HirId,
    ) {
        if (!matches!(kind, FnKind::Closure) && matches!(decl.output, FnRetTy::DefaultReturn(_)))
            || span.ctxt() != body.value.span.ctxt()
            || in_external_macro(cx.sess(), span)
        {
            return;
        }

        let res_ty = cx.typeck_results().expr_ty(&body.value);
        if res_ty.is_unit() || res_ty.is_never() {
            return;
        }

        let expr = if is_async_fn(kind) {
            match get_async_fn_body(cx.tcx, body) {
                Some(e) => e,
                None => return,
            }
        } else {
            &body.value
        };
        lint_implicit_returns(cx, expr, expr.span.ctxt(), None);
    }
}
