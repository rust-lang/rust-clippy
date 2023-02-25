use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::{get_enclosing_block, match_def_path, paths, visitors::for_each_expr, SpanlessEq};
use core::ops::ControlFlow;
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind, PathSegment};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// This lint checks for a call to `reserve` before `extend` on a `Vec` or `VecDeque`.
    /// ### Why is this bad?
    /// Since Rust 1.62, `extend` implicitly calls `reserve`
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
    #[clippy::version = "1.64.0"]
    pub UNNECESSARY_RESERVE,
    pedantic,
    "calling `reserve` before `extend` on a `Vec` or `VecDeque`, when it will be called implicitly"
}

impl_lint_pass!(UnnecessaryReserve => [UNNECESSARY_RESERVE]);

pub struct UnnecessaryReserve {
    msrv: Msrv,
}
impl UnnecessaryReserve {
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryReserve {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !self.msrv.meets(msrvs::EXTEND_IMPLICIT_RESERVE) {
            return;
        }

        if let ExprKind::MethodCall(PathSegment { ident: method, .. }, struct_calling_on, args_a, _) = expr.kind
                && method.name.as_str() == "reserve"
                && acceptable_type(cx, struct_calling_on)
                && let Some(block) = get_enclosing_block(cx, expr.hir_id)
                && let Some(next_stmt_span) = check_extend_method(cx, block, struct_calling_on, &args_a[0])
                && !next_stmt_span.from_expansion()
            {
                span_lint_and_then(
                    cx,
                    UNNECESSARY_RESERVE,
                    next_stmt_span,
                    "unnecessary call to `reserve`",
                    |diag| {
                        diag.span_suggestion(
                            expr.span,
                            "remove this line",
                            String::new(),
                            Applicability::MaybeIncorrect,
                        );
                    }
                );
            }
    }

    extract_msrv_attr!(LateContext);
}

#[must_use]
fn acceptable_type(cx: &LateContext<'_>, struct_calling_on: &rustc_hir::Expr<'_>) -> bool {
    let acceptable_types = [sym::Vec, sym::VecDeque];
    acceptable_types.iter().any(|&acceptable_ty| {
        match cx.typeck_results().expr_ty(struct_calling_on).peel_refs().kind() {
            ty::Adt(def, _) => cx.tcx.is_diagnostic_item(acceptable_ty, def.did()),
            _ => false,
        }
    })
}

#[must_use]
fn check_extend_method(
    cx: &LateContext<'_>,
    block: &Block<'_>,
    struct_expr: &rustc_hir::Expr<'_>,
    args_a: &rustc_hir::Expr<'_>,
) -> Option<rustc_span::Span> {
    let args_a_kind = &args_a.kind;
    let mut read_found = false;
    let mut spanless_eq = SpanlessEq::new(cx);

    let _: Option<!> = for_each_expr(block, |expr| {
        if let ExprKind::MethodCall(_, struct_calling_on, _,_) = expr.kind
            && let Some(expr_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && let ExprKind::MethodCall(PathSegment { ident: method_call_a, .. },..) = args_a_kind
            && method_call_a.name == rustc_span::sym::len
            && match_def_path(cx, expr_def_id, &paths::ITER_EXTEND)
            && acceptable_type(cx, struct_calling_on)
            // Check that both expr are equal
            && spanless_eq.eq_expr(struct_calling_on, struct_expr)
        {
            read_found = true;
        }
        let _: bool = !read_found;
        ControlFlow::Continue(())
    });

    if read_found { Some(block.span) } else { None }
}
