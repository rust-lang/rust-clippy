use clippy_utils::consts::{ConstEvalCtxt, Constant, ConstantSource, FullInt};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, Symbol};
use std::cmp::Ordering;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary calls to `min()` or `max()` in the following cases
    /// - Either both side is constant
    /// - One side is clearly larger than the other, like i32::MIN and an i32 variable
    ///
    /// ### Why is this bad?
    ///
    /// In the aforementioned cases it is not necessary to call `min()` or `max()`
    /// to compare values, it may even cause confusion.
    ///
    /// ### Example
    /// ```no_run
    /// let _ = 0.min(7_u32);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let _ = 0;
    /// ```
    #[clippy::version = "1.81.0"]
    pub UNNECESSARY_MIN_OR_MAX,
    complexity,
    "using 'min()/max()' when there is no need for it"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    name: Symbol,
    recv: &'tcx Expr<'_>,
    arg: &'tcx Expr<'_>,
) {
    let typeck_results = cx.typeck_results();
    let ecx = ConstEvalCtxt::with_env(cx.tcx, cx.typing_env(), typeck_results);
    if let Some(id) = typeck_results.type_dependent_def_id(expr.hir_id)
        && (cx.tcx.is_diagnostic_item(sym::cmp_ord_min, id) || cx.tcx.is_diagnostic_item(sym::cmp_ord_max, id))
    {
        if let Some((left, ConstantSource::Local | ConstantSource::CoreConstant)) = ecx.eval_with_source(recv)
            && let Some((right, ConstantSource::Local | ConstantSource::CoreConstant)) = ecx.eval_with_source(arg)
        {
            let Some(ord) = Constant::partial_cmp(cx.tcx, typeck_results.expr_ty(recv), &left, &right) else {
                return;
            };

            lint(cx, expr, name, recv.span, arg.span, ord);
        } else if let Some(extrema) = detect_extrema(cx, recv) {
            let ord = match extrema {
                Extrema::Minimum => Ordering::Less,
                Extrema::Maximum => Ordering::Greater,
            };
            lint(cx, expr, name, recv.span, arg.span, ord);
        } else if let Some(extrema) = detect_extrema(cx, arg) {
            let ord = match extrema {
                Extrema::Minimum => Ordering::Greater,
                Extrema::Maximum => Ordering::Less,
            };
            lint(cx, expr, name, recv.span, arg.span, ord);
        }
    }
}

fn lint(cx: &LateContext<'_>, expr: &Expr<'_>, name: Symbol, lhs: Span, rhs: Span, order: Ordering) {
    let cmp_str = if order.is_ge() { "smaller" } else { "greater" };

    let suggested_value = if (name == sym::min && order.is_ge()) || (name == sym::max && order.is_le()) {
        snippet(cx, rhs, "..")
    } else {
        snippet(cx, lhs, "..")
    };

    span_lint_and_sugg(
        cx,
        UNNECESSARY_MIN_OR_MAX,
        expr.span,
        format!(
            "`{}` is never {} than `{}` and has therefore no effect",
            snippet(cx, lhs, ".."),
            cmp_str,
            snippet(cx, rhs, "..")
        ),
        "try",
        suggested_value.to_string(),
        Applicability::MachineApplicable,
    );
}

#[derive(Debug)]
enum Extrema {
    Minimum,
    Maximum,
}
fn detect_extrema<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> Option<Extrema> {
    let ty = cx.typeck_results().expr_ty(expr);

    let cv = ConstEvalCtxt::new(cx).eval(expr)?;

    match (cv.int_value(cx.tcx, ty)?, ty.kind()) {
        (FullInt::S(i), &ty::Int(ity)) if i == i128::MIN >> (128 - ity.bit_width()?) => Some(Extrema::Minimum),
        (FullInt::S(i), &ty::Int(ity)) if i == i128::MAX >> (128 - ity.bit_width()?) => Some(Extrema::Maximum),
        (FullInt::U(i), &ty::Uint(uty)) if i == u128::MAX >> (128 - uty.bit_width()?) => Some(Extrema::Maximum),
        (FullInt::U(0), &ty::Uint(_)) => Some(Extrema::Minimum),
        _ => None,
    }
}
