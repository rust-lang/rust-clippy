use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::{is_from_proc_macro, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `T::BITS - x.leading_zeros()` where T and x are of different types
    ///
    /// ### Why is this bad?
    /// Substracting leading_zeros from the number of bits of another type is
    /// a buggy implementation of the `bit_width` method,
    ///
    /// ### Example
    /// ```no_run
    /// let x: u64 = 5;
    /// let bit_width = u32::BITS - x.leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u64 = 5;
    /// let bit_width = u64::BITS - x.leading_zeros();
    /// ```
    #[clippy::version = "1.97.0"]
    pub MISMATCHED_BIT_WIDTH_TYPE,
    suspicious,
    "type mismatch in bit width calculation"
}

impl_lint_pass!(MismatchedBitWidthType => [MISMATCHED_BIT_WIDTH_TYPE]);

pub struct MismatchedBitWidthType {
    msrv: Msrv,
}

impl MismatchedBitWidthType {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl LateLintPass<'_> for MismatchedBitWidthType {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        match expr.kind {
            // `T::BITS - n.leading_zeros()`
            ExprKind::Binary(op, left, right)
                if op.node == BinOpKind::Sub
                    && let ExprKind::MethodCall(leading_zeros, recv, [], _) = right.kind
                    && leading_zeros.ident.name == sym::leading_zeros
                    && let ExprKind::Path(QPath::TypeRelative(hir_ty, segment)) = left.kind
                    && segment.ident.name == sym::BITS
                    && let ty = cx.typeck_results().expr_ty(recv)
                    && cx.typeck_results().node_type(hir_ty.hir_id) != ty
                    && match ty.kind() {
                        // usize::BITS or uint::BITS
                        ty::Uint(_) => true,
                        // NonZero::<uint>::BITS
                        ty::Adt(adt, args)
                            if cx.tcx.is_diagnostic_item(sym::NonZero, adt.did())
                                && let ty::Uint(_) = args.type_at(0).kind() =>
                        {
                            true
                        },
                        _ => return,
                    }
                    && self.msrv.meets(cx, msrvs::BIT_WIDTH)
                    && left.span.eq_ctxt(right.span)
                    && !is_from_proc_macro(cx, expr) =>
            {
                emit(cx, left, ty);
            },
            _ => {},
        }
    }
}

fn emit(cx: &LateContext<'_>, recv: &Expr<'_>, x_ty: Ty<'_>) {
    let app = Applicability::MachineApplicable;

    // Add turbofish syntax for NonZero types
    let mut ty_str = x_ty.to_string();
    if ty_str.contains("NonZero<") {
        ty_str = ty_str.replace("NonZero<", "NonZero::<");
    }

    span_lint_and_sugg(
        cx,
        MISMATCHED_BIT_WIDTH_TYPE,
        recv.span,
        "`T::BITS` should match the type of the value calling `.leading_zeros()`",
        "try",
        format!("{ty_str}::BITS"),
        app,
    );
}
