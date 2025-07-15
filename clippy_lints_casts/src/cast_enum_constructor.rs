use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for casts from an enum tuple constructor to an integer.
    ///
    /// ### Why is this bad?
    /// The cast is easily confused with casting a c-like enum value to an integer.
    ///
    /// ### Example
    /// ```no_run
    /// enum E { X(i32) };
    /// let _ = E::X as usize;
    /// ```
    #[clippy::version = "1.61.0"]
    pub CAST_ENUM_CONSTRUCTOR,
    suspicious,
    "casts from an enum tuple constructor to an integer"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, cast_from: Ty<'_>) {
    if matches!(cast_from.kind(), ty::FnDef(..))
        && let ExprKind::Path(path) = &cast_expr.kind
        && let Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), _) = cx.qpath_res(path, cast_expr.hir_id)
    {
        span_lint(
            cx,
            CAST_ENUM_CONSTRUCTOR,
            expr.span,
            "cast of an enum tuple constructor to an integer",
        );
    }
}
