use clippy_config::Conf;
use clippy_utils::consts::integer_const;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{Msrv, NEXT_MULTIPLE_OF};
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet_with_context;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, MatchSource};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks manual implementation of `next_multiple_of`.
    ///
    /// ### Why is this bad?
    /// This makes code complex and less readable.
    ///
    /// ### Example
    /// ```no_run
    /// let a = 1_u32;
    /// let b = 2_u32;
    ///
    /// let _ = a.div_ceil(b) * b;
    /// let _ = a.div_ceil(b).checked_mul(b);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = 1_u32;
    /// let b = 2_u32;
    ///
    /// let _ = a.next_multiple_of(b);
    /// let _ = a.checked_next_multiple_of(b);
    /// ```
    #[clippy::version = "1.99.0"]
    pub MANUAL_NEXT_MULTIPLE_OF,
    complexity,
    "manually reimplementing `next_multiple_of`"
}

impl_lint_pass!(ManualNextMultipleOf => [MANUAL_NEXT_MULTIPLE_OF]);

pub struct ManualNextMultipleOf {
    msrv: Msrv,
}

impl ManualNextMultipleOf {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualNextMultipleOf {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || !self.msrv.meets(cx, NEXT_MULTIPLE_OF) {
            return;
        }

        let Some(kind) = IntKind::new(cx, expr) else { return };

        if kind.is_signed() {
            // Unstable: <https://github.com/rust-lang/rust/issues/88581>
            return;
        }

        let Some(pat) = Pattern::new(cx, expr, &kind) else {
            return;
        };

        // This lint cannot care about no-op macros, which cannot be detected.
        // Since no-op macros must be modified, it leads to false positives.
        let mut app = Applicability::MaybeIncorrect;

        let method = if kind.is_option() {
            "checked_next_multiple_of"
        } else {
            "next_multiple_of"
        };

        let sugg = match pat {
            Pattern::Arithmetic { a, b, contains_try } => {
                let (a, _) = snippet_with_context(cx, a.span, expr.span.ctxt(), "..", &mut app);
                let (b, _) = snippet_with_context(cx, b.span, expr.span.ctxt(), "..", &mut app);

                if contains_try && !kind.is_option() {
                    format!("{a}.checked_next_multiple_of({b})?")
                } else {
                    format!("{a}.{method}({b})")
                }
            },
            Pattern::DivCeil { a, b } => {
                let (a, _) = snippet_with_context(cx, a.span, expr.span.ctxt(), "..", &mut app);
                let (b, _) = snippet_with_context(cx, b.span, expr.span.ctxt(), "..", &mut app);

                format!("{a}.{method}({b})")
            },
            Pattern::PowerOfTwo { a, b } => {
                let (a, _) = snippet_with_context(cx, a.span, expr.span.ctxt(), "..", &mut app);

                format!("{a}.{method}({b})")
            },
        };

        let msg = format!("manually reimplementing `{method}`");

        span_lint_and_sugg(cx, MANUAL_NEXT_MULTIPLE_OF, expr.span, msg, "try", sugg, app);
    }
}

#[derive(Debug)]
enum IntKind {
    U(ty::UintTy),
    I(ty::IntTy),
    OptU(ty::UintTy),
    OptI(ty::IntTy),
}

impl IntKind {
    fn new<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Self> {
        match cx.typeck_results().expr_ty(expr).kind() {
            ty::Uint(u) => Some(Self::U(*u)),
            ty::Int(i) => Some(Self::I(*i)),
            ty::Adt(def, generic_args)
                if def.is_diag_item(&cx.tcx, sym::Option)
                    && let Some(ty) = generic_args[0].as_type() =>
            {
                match ty.kind() {
                    ty::Uint(u) => Some(Self::OptU(*u)),
                    ty::Int(i) => Some(Self::OptI(*i)),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn is_option(&self) -> bool {
        matches!(self, Self::OptU(_) | Self::OptI(_))
    }

    fn is_signed(&self) -> bool {
        matches!(self, Self::I(_) | Self::OptI(_))
    }

    fn max(&self) -> Option<u128> {
        match self {
            Self::U(u) | Self::OptU(u) => match u {
                // This depends on the machine
                ty::UintTy::Usize => None,
                ty::UintTy::U8 => Some(u128::from(u8::MAX)),
                ty::UintTy::U16 => Some(u128::from(u16::MAX)),
                ty::UintTy::U32 => Some(u128::from(u32::MAX)),
                ty::UintTy::U64 => Some(u128::from(u64::MAX)),
                ty::UintTy::U128 => Some(u128::MAX),
            },
            Self::I(i) | Self::OptI(i) => match i {
                ty::IntTy::Isize => None,
                ty::IntTy::I8 => Some(i8::MAX as u128),
                ty::IntTy::I16 => Some(i16::MAX as u128),
                ty::IntTy::I32 => Some(i32::MAX as u128),
                ty::IntTy::I64 => Some(i64::MAX as u128),
                ty::IntTy::I128 => Some(i128::MAX as u128),
            },
        }
    }
}

enum Pattern<'tcx> {
    Arithmetic {
        a: &'tcx Expr<'tcx>,
        b: &'tcx Expr<'tcx>,
        contains_try: bool,
    },
    PowerOfTwo {
        a: &'tcx Expr<'tcx>,
        b: u128,
    },
    DivCeil {
        a: &'tcx Expr<'tcx>,
        b: &'tcx Expr<'tcx>,
    },
}

impl<'tcx> Pattern<'tcx> {
    fn new(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, kind: &IntKind) -> Option<Self> {
        Self::match_arith_pattern(cx, expr)
            .or_else(|| Self::match_power_of_two_pattern(cx, expr, kind))
            .or_else(|| Self::match_div_ceil_pattern(cx, expr))
    }

    /// Returns `(a, b)` of `a + (b - a % b) % b`.
    fn match_arith_pattern(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Self> {
        // `lhs + rhs`
        let (lhs1, rhs1) = if let Some((recv, [arg])) = unpack_method_call(expr, sym::checked_add) {
            (recv, arg)
        } else {
            unpack_bin_op(expr, BinOpKind::Add)?
        };

        // Only support simple `x.checked_rem(y)?` pattern. Others are too complex.
        // See <https://github.com/rust-lang/rust-clippy/pull/17517#discussion_r3757721544>.
        let mut contains_try = false;
        let mut unpack_rem = |expr| {
            if let Some(expr) = peel_try(expr)
                && let Some((recv, [arg])) = unpack_method_call(expr, sym::checked_rem)
                    .or_else(|| unpack_method_call(expr, sym::checked_rem_euclid))
            {
                contains_try = true;
                Some((recv, arg))
            } else if let Some((recv, [arg])) = unpack_method_call(expr, sym::checked_rem)
                .or_else(|| unpack_method_call(expr, sym::checked_rem_euclid))
                .or_else(|| unpack_method_call(expr, sym::rem_euclid))
            {
                Some((recv, arg))
            } else {
                unpack_bin_op(expr, BinOpKind::Rem)
            }
        };

        // lhs = x % b
        // rhs = a
        let (a, b, x) = if let Some((lhs, rhs)) = unpack_rem(lhs1) {
            (rhs1, rhs, lhs)
        } else
        // lhs = a
        // rhs = x % b
        if let Some((lhs, rhs)) = unpack_rem(rhs1) {
            (lhs1, rhs, lhs)
        } else {
            return None;
        };

        // x = b - a % b
        // Since `a - b % a` can never overflow, checked_sub is not handled and intentionally
        if let Some((lhs, rhs)) = unpack_bin_op(x, BinOpKind::Sub)
            && eq_expr_value(cx, expr.span.ctxt(), lhs, b)
            && let Some((lhs, rhs)) = unpack_rem(rhs)
            && eq_expr_value(cx, expr.span.ctxt(), lhs, a)
            && eq_expr_value(cx, expr.span.ctxt(), rhs, b)
        {
            Some(Self::Arithmetic { a, b, contains_try })
        } else {
            None
        }
    }

    /// Returns `(a, b + 1)` of `(a + b) & !b` where `b + 1` is a power of two.
    fn match_power_of_two_pattern(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, kind: &IntKind) -> Option<Self> {
        //  x & y
        let (lhs, rhs) = unpack_bin_op(expr, BinOpKind::BitAnd)?;

        // (a + b) & c
        let (a, b, c) = if let Some((a, b)) = unpack_bin_op(lhs, BinOpKind::Add) {
            (a, b, rhs)
        } else if let Some((a, b)) = unpack_bin_op(rhs, BinOpKind::Add) {
            (a, b, lhs)
        } else {
            return None;
        };

        // (a + b) & !b
        let c = integer_const(cx, c, expr.span.ctxt())?;
        let (a, b) = if let Some(b) = integer_const(cx, b, expr.span.ctxt())
            && b.checked_add(c) == kind.max()
        {
            (a, b)
        } else if let Some(a) = integer_const(cx, a, expr.span.ctxt())
            && a.checked_add(c) == kind.max()
        {
            (b, a)
        } else {
            return None;
        };

        // Ignores `(a + 0) & !0` and `(a + 0) & !0` because they are useless.
        if 0 < b && b < kind.max().unwrap_or(u128::from(u16::MAX)) && (b + 1).is_power_of_two() {
            Some(Self::PowerOfTwo { a, b: b + 1 })
        } else {
            None
        }
    }

    /// Returns `(a, b)` of `a.div_ceil(b) * b`
    fn match_div_ceil_pattern(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Self> {
        // lhs * rhs
        let (lhs, rhs) = if let Some((recv, [arg])) = unpack_method_call(expr, sym::checked_mul) {
            (recv, arg)
        } else {
            unpack_bin_op(expr, BinOpKind::Mul)?
        };

        // lhs = a.div_ceil(b)
        // rhs = b
        if let Some((a, [b])) = unpack_method_call(lhs, sym::div_ceil)
            && eq_expr_value(cx, expr.span.ctxt(), b, rhs)
        {
            Some(Self::DivCeil { a, b })
        } else
        // lhs = b
        // rhs = a.div_ceil(b)
        // no handling of checked_div_ceil, because does not exist
        if let Some((a, [b])) = unpack_method_call(rhs, sym::div_ceil)
            && eq_expr_value(cx, expr.span.ctxt(), b, lhs)
        {
            Some(Self::DivCeil { a, b })
        } else {
            None
        }
    }
}

/// Returns `(a, b)` of `a ? b`.
fn unpack_bin_op<'tcx>(expr: &'tcx Expr<'tcx>, bin_op_kind: BinOpKind) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(bin_op, lhs, rhs) = expr.kind
        && bin_op.node == bin_op_kind
    {
        Some((lhs, rhs))
    } else {
        None
    }
}

/// Returns `(a, [b, ..])` of `a.method(b, ..)`.
fn unpack_method_call<'tcx>(expr: &'tcx Expr<'tcx>, method: Symbol) -> Option<(&'tcx Expr<'tcx>, &'tcx [Expr<'tcx>])> {
    if let ExprKind::MethodCall(path, receiver, args, _) = expr.kind
        && path.ident.name == method
    {
        Some((receiver, args))
    } else {
        None
    }
}

fn peel_try<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Match(scrutnee, _, MatchSource::TryDesugar(_)) = expr.kind
        && let ExprKind::Call(_, [arg]) = scrutnee.kind
    {
        Some(arg)
    } else {
        None
    }
}
