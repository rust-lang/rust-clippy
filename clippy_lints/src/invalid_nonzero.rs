use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for syntax like `NonZero::new` (including its aliases like `NonZeroU8`,
    /// `NonZeroI32`, etc.) where the argument is a constant `0`.
    ///
    /// ### Why is this bad?
    /// `NonZero::new(0)` always evaluates to `None`. If the result is later
    /// `unwrap`ed, the code is guaranteed to panic at run time.
    ///
    /// ### Example
    /// ```no_run
    /// use std::num::NonZeroU16;
    ///
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::num::NonZeroU16;
    /// let x: Option<NonZeroU16> = None;
    /// ```
    #[clippy::version = "1.100.0"]
    pub INVALID_NONZERO,
    correctness,
    "creating a `NonZero` value from a literal `0`, which always returns `None`"
}

declare_lint_pass!(InvalidNonzero => [INVALID_NONZERO]);

impl<'tcx> LateLintPass<'tcx> for InvalidNonzero {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(func, [arg]) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
            && segment.ident.name == sym::new
            && let nonzero_ty = cx.typeck_results().node_type(ty.hir_id)
            && nonzero_ty.is_diag_item(cx, sym::NonZero)
            && let Some(Constant::Int(0)) = ConstEvalCtxt::new(cx).eval(arg)
        {
            span_lint_and_sugg(
                cx,
                INVALID_NONZERO,
                expr.span,
                "a `NonZero` value crated from a `0` literal will always be `None`",
                "use",
                format!("None::<{nonzero_ty}>"),
                Applicability::MachineApplicable,
            );
        }
    }
}
