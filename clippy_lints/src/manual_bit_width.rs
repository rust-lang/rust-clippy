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
    /// Checks for usage of `T::BITS - x.leading_zeros()`
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
                if left.span.eq_ctxt(right.span)
                    && op.node == BinOpKind::Sub
                    && let ExprKind::MethodCall(leading_zeros, recv, [], _) = right.kind
                    && leading_zeros.ident.name == sym::leading_zeros
                    && let ExprKind::Path(QPath::TypeRelative(hir_ty, segment)) = left.kind
                    && segment.ident.name == sym::BITS
                    && let ty = cx.typeck_results().node_type(hir_ty.hir_id)
                    && let Some(_) = match ty.kind() {
                        // uint::BITS
                        ty::Uint(ty::UintTy::Usize) => Some(cx.tcx.data_layout.pointer_size().bits()),
                        ty::Uint(uint_ty) => uint_ty.bit_width(),
                        // NonZero::<uint>::BITS
                        ty::Adt(adt, args) => {
                            if cx.tcx.is_diagnostic_item(sym::NonZero, adt.did())
                                && let Some(generic_arg) = args.types().next()
                            {
                                match *generic_arg.kind() {
                                    ty::Uint(ty::UintTy::Usize) => Some(cx.tcx.data_layout.pointer_size().bits()),
                                    ty::Uint(uint_ty) => uint_ty.bit_width(),
                                    _ => return,
                                }
                            } else {
                                return;
                            }
                        },
                        ty::Int(_) => {
                            // There is no implementation of `bit_width` for signed integers,
                            // so don't suggest anything.
                            return;
                        },
                        _ => return,
                    }
                    && self.msrv.meets(cx, msrvs::BIT_WIDTH)
                    && !is_from_proc_macro(cx, expr) =>
            {
                emit(cx, recv, expr);
            },
            _ => {},
        }
    }
}

fn emit(cx: &LateContext<'_>, recv: &Expr<'_>, full_expr: &Expr<'_>) {
    let mut app = Applicability::MachineApplicable;
    let (recv_snip, _) = snippet_with_context(cx, recv.span, full_expr.span.ctxt(), "_", &mut app);
    span_lint_and_sugg(
        cx,
        MANUAL_BIT_WIDTH,
        full_expr.span,
        "manually reimplementing `bit_width`",
        "try",
        format!("{recv_snip}.bit_width()"),
        app,
    );
}
