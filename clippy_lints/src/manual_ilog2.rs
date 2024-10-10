use clippy_config::msrvs::{self, Msrv};
use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions like `31 - x.leading_zeros()` or `x.ilog(2)` which are manual
    /// reimplementations of `x.ilog2()`
    /// ### Why is this bad?
    /// Manual reimplementations of `ilog2` increase code complexity for little benefit.
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let log = 31 - x.leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 5;
    /// let log = x.ilog2();
    /// ```
    #[clippy::version = "1.82.0"]
    pub MANUAL_ILOG2,
    complexity,
    "manually reimplementing `ilog2`"
}

pub struct ManualIlog2 {
    msrv: Msrv,
}

impl ManualIlog2 {
    #[must_use]
    pub fn new(conf: &Conf) -> Self {
        Self {
            msrv: conf.msrv.clone(),
        }
    }
}

impl_lint_pass!(ManualIlog2 => [MANUAL_ILOG2]);

impl LateLintPass<'_> for ManualIlog2 {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !self.msrv.meets(msrvs::MANUAL_ILOG2) {
            return;
        }
        let mut applicability = Applicability::MachineApplicable;

        if let ExprKind::Binary(op, left, right) = expr.kind
            && BinOpKind::Sub == op.node
            && let ExprKind::Lit(lit) = left.kind
            && let LitKind::Int(Pu128(val), _) = lit.node
            && let ExprKind::MethodCall(method_name, reciever, _, _) = right.kind
            && method_name.ident.as_str() == "leading_zeros"
        {
            let type_ = cx.typeck_results().expr_ty(reciever);
            let Some(bit_width) = (match type_.kind() {
                ty::Int(itype) => itype.bit_width(),
                ty::Uint(itype) => itype.bit_width(),
                _ => return,
            }) else {
                return;
            };
            if val == u128::from(bit_width) - 1 {
                suggest_change(cx, reciever, expr, &mut applicability);
            }
        }

        if let ExprKind::MethodCall(method_name, reciever, args, _) = expr.kind
            && method_name.ident.as_str() == "ilog"
            && args.len() == 1
            && let ExprKind::Lit(lit) = args[0].kind
            && let LitKind::Int(Pu128(2), _) = lit.node
            && cx.typeck_results().expr_ty(reciever).is_integral()
        {
            suggest_change(cx, reciever, expr, &mut applicability);
        }
    }

    extract_msrv_attr!(LateContext);
}

fn suggest_change(cx: &LateContext<'_>, reciever: &Expr<'_>, full_expr: &Expr<'_>, applicability: &mut Applicability) {
    let sugg = snippet_with_applicability(cx, reciever.span, "..", applicability);
    span_lint_and_sugg(
        cx,
        MANUAL_ILOG2,
        full_expr.span,
        "manually reimplementing `ilog2`",
        "consider using .ilog2()",
        format!("{sugg}.ilog2()"),
        *applicability,
    );
}
