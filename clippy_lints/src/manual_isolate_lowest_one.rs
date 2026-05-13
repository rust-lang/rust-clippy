use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::{SpanlessEq, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::SyntaxContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions like `x & -x` or `x & x.wrapping_neg()`, which are manual
    /// reimplementations of `x.isolate_lowest_one()`.
    ///
    /// ### Why is this bad?
    /// `x.isolate_lowest_one()` is clearer than the bitwise trick. It also avoids the
    /// overflow that occurs when `x == T::MIN` for signed types using the `-` operator,
    /// and preserves non-zero type information for `NonZero<T>`.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let lsb = x & x.wrapping_neg();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 5;
    /// let lsb = x.isolate_lowest_one();
    /// ```
    #[clippy::version = "1.97.0"]
    pub MANUAL_ISOLATE_LOWEST_ONE,
    complexity,
    "manually reimplementing `isolate_lowest_one`"
}

impl_lint_pass!(ManualIsolateLowestOne => [MANUAL_ISOLATE_LOWEST_ONE]);

pub struct ManualIsolateLowestOne {
    msrv: Msrv,
}

impl ManualIsolateLowestOne {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualIsolateLowestOne {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }

        let ExprKind::Binary(op, lhs, rhs) = expr.kind else {
            return;
        };
        if op.node != BinOpKind::BitAnd || lhs.span.from_expansion() || rhs.span.from_expansion() {
            return;
        }

        let ctxt = expr.span.ctxt();
        let recv = is_negation_pair(cx, ctxt, lhs, rhs).or_else(|| is_negation_pair(cx, ctxt, rhs, lhs));
        let Some(recv) = recv else { return };

        if !cx.typeck_results().expr_ty_adjusted(recv).is_integral() {
            return;
        }

        if !self.msrv.meets(cx, msrvs::ISOLATE_LOWEST_ONE) {
            return;
        }

        let mut applicability = Applicability::MachineApplicable;
        let snippet = Sugg::hir_with_context(cx, recv, ctxt, "_", &mut applicability);

        span_lint_and_sugg(
            cx,
            MANUAL_ISOLATE_LOWEST_ONE,
            expr.span,
            "manually reimplementing `isolate_lowest_one`",
            "consider using `.isolate_lowest_one()`",
            format!("{}.isolate_lowest_one()", snippet.maybe_paren()),
            applicability,
        );
    }
}

/// Returns `Some(base)` if `negated` is `-base` or `base.wrapping_neg()` (where `base` is
/// structurally equal to `expected_base`).
fn is_negation_pair<'tcx>(
    cx: &LateContext<'tcx>,
    ctxt: SyntaxContext,
    expected_base: &'tcx Expr<'tcx>,
    negated: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    match negated.kind {
        ExprKind::Unary(UnOp::Neg, inner) if SpanlessEq::new(cx).eq_expr(ctxt, expected_base, inner) => {
            Some(expected_base)
        },
        ExprKind::MethodCall(method, inner, [], _)
            if method.ident.name == sym::wrapping_neg && SpanlessEq::new(cx).eq_expr(ctxt, expected_base, inner) =>
        {
            Some(expected_base)
        },
        _ => None,
    }
}
