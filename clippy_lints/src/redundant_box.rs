use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::approx_ty_size;
use rustc_errors::Applicability;
use rustc_hir::{AmbigArg, Expr, ExprKind, GenericArg, Path, PathSegment, QPath, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.88.0"]
    pub REDUNDANT_BOX,
    nursery,
    "default lint description"
}

// TODO Rename lint as we are not just checking references anymore
declare_lint_pass!(RedundantBox => [REDUNDANT_BOX]);

// TODO could we do everything with only check_ty() xor check_expr()?
impl LateLintPass<'_> for RedundantBox {
    fn check_ty<'tcx>(&mut self, cx: &LateContext<'tcx>, hir_ty: &Ty<'tcx, AmbigArg>) {
        let ty = clippy_utils::ty::ty_from_hir_ty(cx, hir_ty.as_unambig_ty());
        if let Some(boxed_ty) = ty.boxed_ty()
            && is_thin_type(cx, boxed_ty)
        // Extract the contained type for the lint suggestion span
        // TODO is there a simpler way to do this?:
            && let TyKind::Path(QPath::Resolved(_, Path { segments, .. })) = hir_ty.kind
            && let [PathSegment { args: Some(args), .. }] = segments
            && let [GenericArg::Type(ty)] = args.args
        {
            span_lint_and_sugg_(cx, hir_ty.span, ty.span);
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        let ty = cx.typeck_results().expr_ty(expr);
        if let Some(boxed_ty) = ty.boxed_ty()
            && is_thin_type(cx, boxed_ty)
            && let ExprKind::Call(_, &[Expr { span, .. }]) = expr.kind
        {
            span_lint_and_sugg_(cx, expr.span, span);
        }
    }
}

fn is_thin_type<'tcx>(cx: &LateContext<'tcx>, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    //TODO: usize's width will be the host's so lints may be misleading when the intended
    // target is a different architecture. Can/should we do someting about it? Maybe make it
    // configurable?
    ty.is_sized(cx.tcx, cx.typing_env()) && {
        let size = approx_ty_size(cx, ty);
        0 < size && size <= size_of::<usize>() as u64
    }
}

fn span_lint_and_sugg_(cx: &LateContext<'_>, from_span: rustc_span::Span, to_span: rustc_span::Span) {
    span_lint_and_sugg(
        cx,
        REDUNDANT_BOX,
        from_span,
        "TODO: lint msg",
        "Remove Box",
        format!("{}", snippet(cx, to_span, "<default>")),
        Applicability::MachineApplicable,
    );
}
