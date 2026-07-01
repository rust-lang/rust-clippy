use clippy_config::Conf;
use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `T::BITS - x.leading_zeros()` when `x.bit_width()` is available.
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
    #[clippy::version = "1.98.0"]
    pub MANUAL_BIT_WIDTH,
    pedantic,
    "manually reimplementing `bit_width`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `T::BITS - x.leading_zeros()` where T and x are of different types.
    ///
    /// ### Why is this bad?
    /// Substracting `leading_zeros` from the number of bits of another type might be
    /// a buggy implementation of the `bit_width` method.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u64 = 5;
    /// let bit_width = u32::BITS - x.leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u64 = 5;
    /// let bit_width = x.bit_width();
    /// ```
    #[clippy::version = "1.98.0"]
    pub MISMATCHED_BIT_WIDTH_TYPE,
    suspicious,
    "type mismatch in bit width calculation"
}

impl_lint_pass!(ManualBitWidth => [MANUAL_BIT_WIDTH, MISMATCHED_BIT_WIDTH_TYPE]);

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
                    && let right_ty = cx.typeck_results().expr_ty(recv)
                    && let left_ty = cx.typeck_results().node_type(hir_ty.hir_id)
                    && self.msrv.meets(cx, msrvs::BIT_WIDTH)
                    && left.span.eq_ctxt(right.span)
                    && !is_from_proc_macro(cx, expr) =>
            {
                let type_invariant = match right_ty.kind() {
                    // int::BITS or uint::BITS
                    ty::Uint(_) => IntTypeVariant::UnsignedInt,
                    ty::Int(_) => IntTypeVariant::SignedInt,
                    // NonZero::<int/uint>::BITS
                    ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::NonZero, adt.did()) => {
                        match args.type_at(0).kind() {
                            ty::Uint(_) => IntTypeVariant::NonZeroUnsigned,
                            ty::Int(_) => IntTypeVariant::NonZeroSigned,
                            _ => return,
                        }
                    },
                    _ => return,
                };

                if let Some(left_width) = get_bit_width(cx, left_ty)
                    && let Some(right_width) = get_bit_width(cx, right_ty)
                    && left_width == right_width
                {
                    // manual implementation of bit_width
                    emit_manual_bit_width(cx, recv, expr, type_invariant);
                } else {
                    // mismatched calling types
                    emit_type_mismatch(cx, recv, expr, type_invariant, right_ty);
                }
            },
            _ => {},
        }
    }
}

fn get_bit_width(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<u64> {
    // num.bit_width() return None for `usize` and `isize` so we need this wrapper
    let size_safe_bit_width = |x: Option<u64>| x.or_else(|| Some(cx.tcx.data_layout.pointer_size().bits()));

    match ty.kind() {
        // int::BITS or uint::BITS
        ty::Uint(num) => size_safe_bit_width(num.bit_width()),
        ty::Int(num) => size_safe_bit_width(num.bit_width()),
        // NonZero::<int/uint>::BITS
        ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::NonZero, adt.did()) => {
            let arg = args.type_at(0);
            match arg.kind() {
                ty::Uint(num) => size_safe_bit_width(num.bit_width()),
                ty::Int(num) => size_safe_bit_width(num.bit_width()),
                _ => None,
            }
        },
        _ => None,
    }
}

fn emit_manual_bit_width(
    cx: &LateContext<'_>,
    recv: &Expr<'_>,
    full_expr: &Expr<'_>,
    int_type_invariant: IntTypeVariant,
) {
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

fn emit_type_mismatch(
    cx: &LateContext<'_>,
    recv: &Expr<'_>,
    full_expr: &Expr<'_>,
    int_type_invariant: IntTypeVariant,
    x_ty: Ty<'_>,
) {
    span_lint_and_then(
        cx,
        MISMATCHED_BIT_WIDTH_TYPE,
        full_expr.span,
        "possible buggy implementation of `bit_width`",
        |diag| {
            diag.note("in order to calculate the bit width, `T::BITS` should match the type of the value calling `.leading_zeros()`");

            let mut app = Applicability::MaybeIncorrect;
            let (recv_snip, _) = snippet_with_context(cx, recv.span, full_expr.span.ctxt(), "_", &mut app);
            let suggestion = int_type_invariant.suggestion();

            diag.span_suggestion_verbose(
                full_expr.span,
                format!("if you meant to use `{x_ty}::BITS`, use"),
                format!("{recv_snip}{suggestion}"),
                app,
            );
        },
    );
}
