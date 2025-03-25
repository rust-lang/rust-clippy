use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::ty::adt_def_id;
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{SpanlessEq, get_enclosing_block, match_def_path, paths};
use core::ops::ControlFlow;
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// This lint checks for a call to `reserve` before `extend` on a `Vec` or `VecDeque`.
    /// ### Why is this bad?
    /// `extend` implicitly calls `reserve`
    ///
    /// ### Example
    /// ```rust
    /// let mut vec: Vec<usize> = vec![];
    /// let array: &[usize] = &[1, 2];
    /// vec.reserve(array.len());
    /// vec.extend(array);
    /// ```
    /// Use instead:
    /// ```rust
    /// let mut vec: Vec<usize> = vec![];
    /// let array: &[usize] = &[1, 2];
    /// vec.extend(array);
    /// ```
    #[clippy::version = "1.86.0"]
    pub UNNECESSARY_RESERVE,
    complexity,
    "calling `reserve` before `extend` on a `Vec` or `VecDeque`, when it will be called implicitly"
}

impl_lint_pass!(UnnecessaryReserve => [UNNECESSARY_RESERVE]);

pub struct UnnecessaryReserve {
    msrv: Msrv,
}
impl UnnecessaryReserve {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv.clone(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryReserve {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !self.msrv.meets(msrvs::EXTEND_IMPLICIT_RESERVE) {
            return;
        }

        if let ExprKind::MethodCall(method, receiver, [arg], _) = expr.kind
            && method.ident.name.as_str() == "reserve"
            && acceptable_type(cx, receiver)
            && let Some(block) = get_enclosing_block(cx, expr.hir_id)
            && let Some(next_stmt_span) = check_extend_method(cx, block, receiver, arg)
            && !next_stmt_span.from_expansion()
        {
            let stmt_span = cx
                .tcx
                .hir()
                .parent_id_iter(expr.hir_id)
                .next()
                .map_or(expr.span, |parent| cx.tcx.hir().span(parent));

            span_lint_and_then(
                cx,
                UNNECESSARY_RESERVE,
                next_stmt_span,
                "unnecessary call to `reserve`",
                |diag| {
                    diag.span_suggestion(
                        stmt_span,
                        "remove this line",
                        String::new(),
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }
    }

    extract_msrv_attr!(LateContext);
}

fn acceptable_type(cx: &LateContext<'_>, struct_calling_on: &Expr<'_>) -> bool {
    adt_def_id(cx.typeck_results().expr_ty_adjusted(struct_calling_on))
        .is_some_and(|did| matches!(cx.tcx.get_diagnostic_name(did), Some(sym::Vec | sym::VecDeque)))
}

#[must_use]
fn check_extend_method<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    struct_expr: &Expr<'tcx>,
    arg: &Expr<'tcx>,
) -> Option<rustc_span::Span> {
    let mut found_reserve = false;
    let mut spanless_eq = SpanlessEq::new(cx).deny_side_effects();

    for_each_expr(cx, block, |expr: &Expr<'tcx>| {
        if !found_reserve {
            found_reserve = expr.hir_id == arg.hir_id;
            return ControlFlow::Continue(());
        }

        if let ExprKind::MethodCall(_method, struct_calling_on, _, _) = expr.kind
            && let Some(expr_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && let ExprKind::MethodCall(len_method, ..) = arg.kind
            && len_method.ident.name == sym::len
            && match_def_path(cx, expr_def_id, &paths::ITER_EXTEND)
            && acceptable_type(cx, struct_calling_on)
            && spanless_eq.eq_expr(struct_calling_on, struct_expr)
        {
            return ControlFlow::Break(block.span);
        }
        ControlFlow::Continue(())
    })
}
