use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `T::BITS - x.leading_zeros()` where T matches UINT or NonZero<UINT>
    /// when `x.bit_width()` is available.
    ///
    /// ### Why is this bad?
    ///  Manual reimplementations of `bit_width` increase code complexity for little benefit.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let bit_width = u32::BITS - x.leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 5;
    /// let bit_width = x.bit_width();
    /// ```
    #[clippy::version = "1.97.0"]
    pub MANUAL_BIT_WIDTH,
    pedantic,
    "manually reimplementing `bit_width`"
}

impl_lint_pass!(ManualBitWidth => [MANUAL_BIT_WIDTH]);

enum IntTypeVariant {
    UnsignedInt,
    SignedInt,
    NonZeroUnsigned,
    NonZeroSigned,
}

impl IntTypeVariant {
    fn suggestion(self) -> &'static str {
        match self {
            Self::UnsignedInt => ".bit_width()",
            Self::SignedInt => ".cast_unsigned().bit_width()",
            Self::NonZeroUnsigned => ".bit_width().get()",
            Self::NonZeroSigned => ".cast_unsigned().bit_width().get()",
        }
    }
}

pub struct ManualBitWidth {
    msrv: Msrv,
}

impl ManualBitWidth {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl LateLintPass<'_> for ManualBitWidth {
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
                    && let ty = cx.typeck_results().node_type(hir_ty.hir_id)
                    // TODO: If the types don't match, then this might be a buggy implementation of bit_width. Consider adding a separate, suspicious lint for that.
                    // See: https://github.com/rust-lang/rust-clippy/pull/16902#discussion_r3412814226
                    && cx.typeck_results().expr_ty(recv) == ty
                    && let int_type_invariant = match ty.kind() {
                        // usize::BITS or uint::BITS
                        ty::Uint(_) => IntTypeVariant::UnsignedInt,
                        ty::Int(_) => IntTypeVariant::SignedInt,
                        // NonZero::<uint>::BITS
                        ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::NonZero, adt.did()) => {
                            match args.type_at(0).kind() {
                                ty::Uint(_) => IntTypeVariant::NonZeroUnsigned,
                                ty::Int(_) => IntTypeVariant::NonZeroSigned,
                                _ => return,
                            }
                        },
                        _ => return,
                    }
                    && self.msrv.meets(cx, msrvs::BIT_WIDTH)
                    && left.span.eq_ctxt(right.span)
                    && !is_from_proc_macro(cx, expr) =>
            {
                emit(cx, recv, expr, int_type_invariant);
            },
            _ => {},
        }
    }
}

fn emit(cx: &LateContext<'_>, recv: &Expr<'_>, full_expr: &Expr<'_>, int_type_invariant: IntTypeVariant) {
    let mut app = Applicability::MachineApplicable;
    let (recv_snip, _) = snippet_with_context(cx, recv.span, full_expr.span.ctxt(), "_", &mut app);

    let suggestion = int_type_invariant.suggestion();

    span_lint_and_sugg(
        cx,
        MANUAL_BIT_WIDTH,
        full_expr.span,
        "manual implementation of `bit_width`",
        "try",
        format!("{recv_snip}{suggestion}"),
        app,
    );
}
