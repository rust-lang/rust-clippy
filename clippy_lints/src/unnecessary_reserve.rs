use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{match_def_path, meets_msrv, msrvs, paths, visitors::expr_visitor_no_bodies};
use rustc_errors::Applicability;
use rustc_hir::{intravisit::Visitor, Block, ExprKind, QPath, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_semver::RustcVersion;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// This lint checks for a call to `reserve` before `extend` on a `Vec` or `VecDeque`.
    /// ### Why is this bad?
    /// Since Rust 1.62, `extend` implicitly calls `reserve`
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

pub struct UnnecessaryReserve {
    msrv: Option<RustcVersion>,
}

impl UnnecessaryReserve {
    #[must_use]
    pub fn new(msrv: Option<RustcVersion>) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(UnnecessaryReserve => [UNNECESSARY_RESERVE]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryReserve {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &Block<'tcx>) {
        if !meets_msrv(self.msrv, msrvs::UNNECESSARY_RESERVE) {
            return;
        }

        for (idx, stmt) in block.stmts.iter().enumerate() {
            if let StmtKind::Semi(semi_expr) = stmt.kind
                && let ExprKind::MethodCall(_, [struct_calling_on, _], _) = semi_expr.kind
                && let Some(expr_def_id) = cx.typeck_results().type_dependent_def_id(semi_expr.hir_id)
                && (match_def_path(cx, expr_def_id, &paths::VEC_RESERVE) ||
                    match_def_path(cx, expr_def_id, &paths::VEC_DEQUE_RESERVE))
                && acceptable_type(cx, struct_calling_on)
                && let Some(next_stmt_span) = check_extend_method(cx, block, idx, struct_calling_on)
                && !next_stmt_span.from_expansion()
            {
                span_lint_and_then(
                    cx,
                    UNNECESSARY_RESERVE,
                    next_stmt_span,
                    "unnecessary call to `reserve`",
                    |diag| {
                        diag.span_suggestion(
                            semi_expr.span,
                            "remove this line",
                            String::new(),
                            Applicability::MaybeIncorrect,
                        );
                    }
                );
            }
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
    idx: usize,
    struct_expr: &rustc_hir::Expr<'_>,
) -> Option<rustc_span::Span> {
    let mut read_found = false;
    let next_stmt_span;

    let mut visitor = expr_visitor_no_bodies(|expr| {
        if let ExprKind::MethodCall(_, [struct_calling_on, _], _) = expr.kind
            && let Some(expr_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && match_def_path(cx, expr_def_id, &paths::ITER_EXTEND)
            && acceptable_type(cx, struct_calling_on)
            && equal_ident(struct_calling_on, struct_expr)
        {
            read_found = true;
        }
        !read_found
    });

    if idx == block.stmts.len() - 1 {
        if let Some(e) = block.expr {
            visitor.visit_expr(e);
            next_stmt_span = e.span;
        } else {
            return None;
        }
    } else {
        let next_stmt = &block.stmts[idx + 1];
        visitor.visit_stmt(next_stmt);
        next_stmt_span = next_stmt.span;
    }
    drop(visitor);

    if read_found {
        return Some(next_stmt_span);
    }

    None
}

#[must_use]
fn equal_ident(left: &rustc_hir::Expr<'_>, right: &rustc_hir::Expr<'_>) -> bool {
    fn ident_name(expr: &rustc_hir::Expr<'_>) -> Option<rustc_span::Symbol> {
        if let ExprKind::Path(QPath::Resolved(None, inner_path)) = expr.kind
            && let [inner_seg] = inner_path.segments
        {
            return Some(inner_seg.ident.name);
        }
        None
    }

    ident_name(left) == ident_name(right)
}
