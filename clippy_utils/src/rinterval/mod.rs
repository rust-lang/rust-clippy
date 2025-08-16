//! A module for modeling the behavior of std functions using integer
//! arithmetic.
//!
//! Currently, only integer intervals are supported, but floating point
//! intervals can be added later.

mod arithmetic;
mod bits;
mod iinterval;

pub use arithmetic::*;
pub use iinterval::*;

use rustc_ast::LitKind;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, Expr, ExprKind, HirId, PathSegment, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::{IntTy, Ty, TyKind, TypeckResults, UintTy};
use rustc_span::Symbol;

use crate::consts::{ConstEvalCtxt, Constant};
use crate::sym;
use crate::ty::is_type_diagnostic_item;

pub struct IntervalCtxt<'c, 'cx> {
    cx: &'c LateContext<'cx>,
    typeck: &'cx TypeckResults<'cx>,
    const_eval: ConstEvalCtxt<'cx>,

    arth: Arithmetic,
    isize_ty: IntType,

    cache: FxHashMap<HirId, Value>,
}

#[derive(Clone, Debug)]
enum Value {
    /// The value is of the never type.
    Never,
    /// The value is an integer. The set of possible values is represented by an interval.
    Int(IInterval),

    /// Any value of an unknown type. We truly know nothing about this value.
    Unknown,
}
impl Value {
    pub fn as_int(&self, ty: IntType) -> Option<IInterval> {
        match self {
            Value::Int(interval) if interval.ty == ty => Some(interval.clone()),
            // coerce never to an empty interval
            Value::Never => Some(IInterval::empty(ty)),
            _ => None,
        }
    }
    pub fn int_or_unknown<E>(interval: Result<IInterval, E>) -> Self {
        match interval {
            Ok(interval) => Value::Int(interval),
            Err(_) => Value::Unknown,
        }
    }
    pub fn unknown_to(self, f: impl FnOnce() -> Self) -> Self {
        match self {
            Value::Unknown => f(),
            _ => self,
        }
    }
    pub fn unknown_to_full(self, ty: IntType) -> Self {
        match self {
            Value::Unknown => Value::Int(IInterval::full(ty)),
            _ => self,
        }
    }
    pub fn full(ty: IntType) -> Self {
        Value::Int(IInterval::full(ty))
    }

    pub fn union(self, other: Self) -> Self {
        match (self, other) {
            (Value::Never, value) | (value, Value::Never) => value,

            (_, Value::Unknown) | (Value::Unknown, _) => Value::Unknown,

            (Value::Int(a), Value::Int(b)) => {
                if let Some(interval) = a.hull(&b) {
                    Value::Int(interval)
                } else {
                    // This really shouldn't happen, but just in case
                    Value::Unknown
                }
            },
        }
    }
}
impl From<IInterval> for Value {
    fn from(interval: IInterval) -> Self {
        Value::Int(interval)
    }
}

trait Extensions {
    fn or_full(self, ty: IntType) -> Value;
    fn or_unknown(self) -> Value;
}
impl Extensions for Option<IInterval> {
    fn or_full(self, ty: IntType) -> Value {
        match self {
            Some(interval) if interval.ty == ty => Value::Int(interval),
            _ => Value::Int(IInterval::full(ty)),
        }
    }
    fn or_unknown(self) -> Value {
        match self {
            Some(interval) => Value::Int(interval),
            None => Value::Unknown,
        }
    }
}
impl<E> Extensions for Result<IInterval, E> {
    fn or_full(self, ty: IntType) -> Value {
        match self {
            Ok(interval) if interval.ty == ty => Value::Int(interval),
            _ => Value::Int(IInterval::full(ty)),
        }
    }
    fn or_unknown(self) -> Value {
        match self {
            Ok(interval) => Value::Int(interval),
            Err(_) => Value::Unknown,
        }
    }
}

impl<'c, 'cx> IntervalCtxt<'c, 'cx> {
    pub fn new(cxt: &'c LateContext<'cx>) -> Self {
        let isize_int = rustc_abi::HasDataLayout::data_layout(&cxt.tcx).ptr_sized_integer();
        let isize_ty = match isize_int {
            rustc_abi::Integer::I8 => IntType::I8,
            rustc_abi::Integer::I16 => IntType::I16,
            rustc_abi::Integer::I32 => IntType::I32,
            rustc_abi::Integer::I64 => IntType::I64,
            rustc_abi::Integer::I128 => IntType::I128,
        };

        IntervalCtxt {
            cx: cxt,
            typeck: cxt.typeck_results(),
            const_eval: ConstEvalCtxt::new(cxt),

            arth: Arithmetic { checked: false },
            isize_ty,

            cache: FxHashMap::default(),
        }
    }

    /// Evaluates an expression to an integer interval.
    ///
    /// If the given expression is not of a supported integer type, None is
    /// returned.
    pub fn eval_int(&mut self, expr: &Expr<'cx>) -> Option<IInterval> {
        if let Value::Int(interval) = self.eval(expr) {
            Some(interval)
        } else {
            None
        }
    }

    fn eval(&mut self, expr: &Expr<'cx>) -> Value {
        let cache_key = expr.hir_id;
        // Check the cache first.
        if let Some(interval) = self.cache.get(&cache_key) {
            return interval.clone();
        }

        // we only care about values, so ignore borrows
        let expr = expr.peel_borrows();

        let expr_ty = self.typeck.expr_ty(expr);
        let Some(ty) = self.to_int_type(expr_ty) else {
            return if expr_ty.is_never() {
                Value::Never
            } else {
                Value::Unknown
            };
        };

        let expr = self.cx.expr_or_init(expr);
        let value = self.eval_int_ty(expr, ty).unknown_to_full(ty);

        // Cache the result.
        self.cache.insert(expr.hir_id, value.clone());
        self.cache.insert(cache_key, value.clone());

        value
    }
    /// Evaluates an expression to an integer interval of the given type.
    fn eval_int_ty(&mut self, expr: &Expr<'cx>, ty: IntType) -> Value {
        let expr_ty = self.typeck.expr_ty(expr);
        if expr_ty.is_never() {
            // If the expression is never, we can return an empty interval.
            return IInterval::empty(ty).into();
        }

        match expr.kind {
            ExprKind::Lit(lit) => {
                return self.literal(&lit.node, ty);
            },
            ExprKind::Binary(op, lhs, rhs) => {
                return self.binary_op(op.node, lhs, rhs);
            },
            ExprKind::Unary(op, operand) => {
                if let Some(operand_interval) = self.eval(operand).as_int(ty) {
                    return self.unary_op(op, &operand_interval);
                }
            },
            ExprKind::Cast(expr, _) => {
                if let Value::Int(expr_interval) = self.eval(expr) {
                    return Arithmetic::cast_as(&expr_interval, ty).or_full(ty);
                }
            },

            // For conditional expressions, we evaluate all branches and
            // return the hull (union) of them.
            //
            // No attempt is made at trimming down on branches. All branches
            // are assumed to be reachable.
            ExprKind::If(_cond, if_true, Some(if_false)) => {
                return Self::branches(&[if_true, if_false], |e| self.eval(*e));
            },
            ExprKind::Match(_expr, arms, _) => {
                return Self::branches(arms, |arm| self.eval(arm.body));
            },

            // Known methods and functions of integer types.
            ExprKind::MethodCall(path, self_arg, args, _) => return self.method_call(path, self_arg, args, ty),

            _ => {},
        }

        // if all else fails, try to evaluate the expression using const eval
        self.const_eval(expr, ty).unknown_to_full(ty)
    }

    fn branches<T>(branches: &[T], mut get_value: impl FnMut(&T) -> Value) -> Value {
        let mut combined = Value::Never;

        for branch in branches {
            let branch_value = get_value(branch);

            if matches!(branch_value, Value::Unknown) {
                // once unknown, always unknown
                return Value::Unknown;
            }

            combined = combined.union(branch_value);
        }

        combined
    }

    fn literal(&self, lit: &LitKind, ty: IntType) -> Value {
        match lit {
            LitKind::Int(n, _) => Self::u128_repr_to_interval(n.get(), ty).or_full(ty),
            _ => Value::Unknown,
        }
    }
    fn binary_op(&mut self, op: BinOpKind, l_expr: &Expr<'cx>, r_expr: &Expr<'cx>) -> Value {
        let Value::Int(lhs) = &self.eval(l_expr) else {
            return Value::Unknown;
        };

        // The pattern `x * x` is quite common and will always result in a
        // positive value (absent overflow). To support this, special handling
        // is required.
        if matches!(op, BinOpKind::Mul) && self.is_same_variable(l_expr, r_expr) {
            return Arithmetic::wrapping_pow(lhs, &IInterval::single_unsigned(IntType::U32, 2)).or_unknown();
        }

        let Value::Int(rhs) = &self.eval(r_expr) else {
            return Value::Unknown;
        };

        match op {
            BinOpKind::Add => self.arth.add(lhs, rhs),
            BinOpKind::Sub => self.arth.sub(lhs, rhs),
            BinOpKind::Mul => self.arth.mul(lhs, rhs),
            BinOpKind::Div => self.arth.div(lhs, rhs),
            BinOpKind::Rem => self.arth.rem(lhs, rhs),
            BinOpKind::BitAnd => Arithmetic::and(lhs, rhs),
            BinOpKind::BitOr => Arithmetic::or(lhs, rhs),
            BinOpKind::BitXor => Arithmetic::xor(lhs, rhs),
            BinOpKind::Shl => self.arth.shl(lhs, rhs),
            BinOpKind::Shr => self.arth.shr(lhs, rhs),
            _ => return Value::Unknown,
        }
        .or_unknown()
    }
    fn unary_op(&mut self, op: UnOp, value: &IInterval) -> Value {
        match op {
            UnOp::Neg => self.arth.neg(value).or_unknown(),
            UnOp::Not => Arithmetic::not(value).or_unknown(),
            UnOp::Deref => {
                // Deref doesn't really make sense for numbers, but it does make
                // sense for references to numbers. Assuming that the value is
                // indeed a reference to a number, we can just return the value
                // of the number.
                Value::Int(value.clone())
            },
        }
    }

    /// Calls to methods that returns an integer.
    fn method_call(
        &mut self,
        path: &PathSegment<'_>,
        self_arg: &Expr<'cx>,
        args: &[Expr<'cx>],
        ret_ty: IntType,
    ) -> Value {
        match args {
            [] => {
                let f: Option<fn(&Arithmetic, &IInterval) -> ArithResult> = match path.ident.name {
                    sym::neg => Some(Arithmetic::neg),
                    sym::checked_neg => Some(|_, x| Arithmetic::strict_neg(x)),
                    sym::saturating_neg => Some(|_, x| Arithmetic::saturating_neg(x)),
                    sym::strict_neg => Some(|_, x| Arithmetic::strict_neg(x)),
                    sym::wrapping_neg => Some(|_, x| Arithmetic::wrapping_neg(x)),

                    sym::isqrt | sym::checked_isqrt => Some(|_, x| Arithmetic::isqrt(x)),
                    sym::ilog2 | sym::checked_ilog2 => Some(|_, x| Arithmetic::ilog2(x)),
                    sym::ilog10 | sym::checked_ilog10 => Some(|_, x| Arithmetic::ilog10(x)),

                    sym::abs => Some(Arithmetic::abs),
                    sym::checked_abs => Some(|_, x| Arithmetic::strict_abs(x)),
                    sym::saturating_abs => Some(|_, x| Arithmetic::saturating_abs(x)),
                    sym::strict_abs => Some(|_, x| Arithmetic::strict_abs(x)),
                    sym::wrapping_abs => Some(|_, x| Arithmetic::wrapping_abs(x)),
                    sym::unsigned_abs => Some(|_, x| Arithmetic::unsigned_abs(x)),

                    sym::not => Some(|_, x| Arithmetic::not(x)),

                    sym::signum => Some(|_, x| Arithmetic::signum(x)),

                    sym::next_power_of_two => Some(Arithmetic::next_power_of_two),
                    sym::checked_next_power_of_two => Some(|_, x| Arithmetic::strict_next_power_of_two(x)),
                    sym::wrapping_next_power_of_two => Some(|_, x| Arithmetic::wrapping_next_power_of_two(x)),

                    sym::cast_signed => Some(|_, x| Arithmetic::cast_signed(x)),
                    sym::cast_unsigned => Some(|_, x| Arithmetic::cast_unsigned(x)),

                    sym::leading_zeros => Some(|_, x| Arithmetic::leading_zeros(x)),
                    sym::leading_ones => Some(|_, x| Arithmetic::leading_ones(x)),
                    sym::trailing_zeros => Some(|_, x| Arithmetic::trailing_zeros(x)),
                    sym::trailing_ones => Some(|_, x| Arithmetic::trailing_ones(x)),
                    sym::count_ones => Some(|_, x| Arithmetic::count_ones(x)),
                    sym::count_zeros => Some(|_, x| Arithmetic::count_zeros(x)),

                    _ => None,
                };

                if let Some(f) = f
                    && let Value::Int(self_arg) = self.eval(self_arg)
                {
                    return f(&self.arth, &self_arg).or_full(ret_ty);
                }
            },

            [arg1] => {
                let f: Option<fn(&Arithmetic, &IInterval, &IInterval) -> ArithResult> = match path.ident.name {
                    sym::add => Some(Arithmetic::add),
                    sym::checked_add => Some(|_, l, r| Arithmetic::strict_add(l, r)),
                    sym::saturating_add => Some(|_, l, r| Arithmetic::saturating_add(l, r)),
                    sym::strict_add => Some(|_, l, r| Arithmetic::strict_add(l, r)),
                    sym::wrapping_add => Some(|_, l, r| Arithmetic::wrapping_add(l, r)),

                    sym::sub => Some(Arithmetic::sub),
                    sym::checked_sub => Some(|_, l, r| Arithmetic::strict_sub(l, r)),
                    sym::saturating_sub => Some(|_, l, r| Arithmetic::saturating_sub(l, r)),
                    sym::strict_sub => Some(|_, l, r| Arithmetic::strict_sub(l, r)),
                    sym::wrapping_sub => Some(|_, l, r| Arithmetic::wrapping_sub(l, r)),

                    sym::mul => Some(Arithmetic::mul),
                    sym::checked_mul => Some(|_, l, r| Arithmetic::strict_mul(l, r)),
                    sym::saturating_mul => Some(|_, l, r| Arithmetic::saturating_mul(l, r)),
                    sym::strict_mul => Some(|_, l, r| Arithmetic::strict_mul(l, r)),
                    sym::wrapping_mul => Some(|_, l, r| Arithmetic::wrapping_mul(l, r)),

                    sym::div => Some(Arithmetic::div),
                    sym::checked_div => Some(|_, l, r| Arithmetic::strict_div(l, r)),
                    sym::saturating_div => Some(|_, l, r| Arithmetic::saturating_div(l, r)),
                    sym::strict_div => Some(|_, l, r| Arithmetic::strict_div(l, r)),
                    sym::wrapping_div => Some(|_, l, r| Arithmetic::wrapping_div(l, r)),

                    sym::div_euclid => Some(|_, l, r| Arithmetic::strict_div_euclid(l, r)),
                    sym::checked_div_euclid => Some(|_, l, r| Arithmetic::strict_div_euclid(l, r)),
                    sym::wrapping_div_euclid => Some(|_, l, r| Arithmetic::wrapping_div_euclid(l, r)),

                    sym::div_ceil => Some(|_, l, r| Arithmetic::div_ceil(l, r)),

                    sym::rem => Some(Arithmetic::rem),
                    sym::checked_rem => Some(|_, l, r| Arithmetic::strict_rem(l, r)),
                    sym::strict_rem => Some(|_, l, r| Arithmetic::strict_rem(l, r)),
                    sym::wrapping_rem => Some(|_, l, r| Arithmetic::wrapping_rem(l, r)),

                    sym::rem_euclid => Some(Arithmetic::rem_euclid),
                    sym::checked_rem_euclid => Some(|_, l, r| Arithmetic::strict_rem_euclid(l, r)),
                    sym::strict_rem_euclid => Some(|_, l, r| Arithmetic::strict_rem_euclid(l, r)),
                    sym::wrapping_rem_euclid => Some(|_, l, r| Arithmetic::wrapping_rem_euclid(l, r)),

                    sym::midpoint => Some(|_, l, r| Arithmetic::midpoint(l, r)),

                    sym::abs_diff => Some(|_, l, r| Arithmetic::abs_diff(l, r)),

                    sym::next_multiple_of => Some(|_, l, r| Arithmetic::strict_next_multiple_of(l, r)),
                    sym::checked_next_multiple_of => Some(|_, l, r| Arithmetic::strict_next_multiple_of(l, r)),

                    sym::pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::checked_pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::saturating_pow => Some(|_, l, r| Arithmetic::saturating_pow(l, r)),
                    sym::strict_pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::wrapping_pow => Some(|_, l, r| Arithmetic::wrapping_pow(l, r)),

                    sym::ilog | sym::checked_ilog => Some(|_, l, r| Arithmetic::ilog(l, r)),

                    sym::min => Some(|_, l, r| Arithmetic::min(l, r)),
                    sym::max => Some(|_, l, r| Arithmetic::max(l, r)),

                    sym::bitand => Some(|_, l, r| Arithmetic::and(l, r)),
                    sym::bitor => Some(|_, l, r| Arithmetic::or(l, r)),
                    sym::bitxor => Some(|_, l, r| Arithmetic::xor(l, r)),

                    sym::shl => Some(Arithmetic::shl),
                    sym::checked_shl => Some(|_, l, r| Arithmetic::strict_shl(l, r)),
                    sym::strict_shl => Some(|_, l, r| Arithmetic::strict_shl(l, r)),
                    sym::wrapping_shl => Some(|_, l, r| Arithmetic::wrapping_shl(l, r)),
                    sym::unbounded_shl => Some(|_, l, r| Arithmetic::unbounded_shl(l, r)),

                    sym::shr => Some(Arithmetic::shr),
                    sym::checked_shr => Some(|_, l, r| Arithmetic::strict_shr(l, r)),
                    sym::strict_shr => Some(|_, l, r| Arithmetic::strict_shr(l, r)),
                    sym::wrapping_shr => Some(|_, l, r| Arithmetic::wrapping_shr(l, r)),
                    sym::unbounded_shr => Some(|_, l, r| Arithmetic::unbounded_shr(l, r)),

                    _ => None,
                };

                if let Some(f) = f
                    && let Value::Int(self_arg) = self.eval(self_arg)
                    && let Value::Int(arg1) = self.eval(arg1)
                {
                    return f(&self.arth, &self_arg, &arg1).or_full(ret_ty);
                }
            },

            [arg1, arg2] => {
                let f: Option<fn(&Arithmetic, &IInterval, &IInterval, &IInterval) -> ArithResult> =
                    match path.ident.name {
                        sym::clamp => Some(|_, a, b, c| Arithmetic::max(b, &Arithmetic::min(a, c)?)),

                        _ => None,
                    };

                if let Some(f) = f
                    && let Value::Int(self_arg) = self.eval(self_arg)
                    && let Value::Int(arg1) = self.eval(arg1)
                    && let Value::Int(arg2) = self.eval(arg2)
                {
                    return f(&self.arth, &self_arg, &arg1, &arg2).or_full(ret_ty);
                }
            },
            _ => {},
        }

        /// A list of supported `Option<T>` methods
        const OPTION_METHODS: &[Symbol] = &[
            sym::unwrap,
            sym::unwrap_unchecked,
            sym::unwrap_or,
            sym::unwrap_or_default,
            sym::expect,
        ];

        if OPTION_METHODS.contains(&path.ident.name) {
            // It's highly likely that self is an option, so check to the type
            // to verify that.
            let self_ty = self.typeck.expr_ty(self_arg);
            let is_option = is_type_diagnostic_item(self.cx, self_ty, sym::Option);

            if is_option || true {
                let self_value = self.eval_int_ty(self_arg, ret_ty);

                match path.ident.name {
                    sym::unwrap | sym::unwrap_unchecked | sym::expect => {
                        // these are all the same in that they return the Some value
                        return self_value;
                    },
                    sym::unwrap_or_default => {
                        // the default value of all integer types is 0, so we can
                        // evaluate the Some value and add 0 to it.
                        let zero = if ret_ty.is_signed() {
                            IInterval::single_signed(ret_ty, 0)
                        } else {
                            IInterval::single_unsigned(ret_ty, 0)
                        };
                        return self_value.union(zero.into());
                    },
                    sym::unwrap_or => {
                        // the default value is given as the second argument
                        let Some(arg0) = args.get(0) else {
                            // this really shouldn't happen, but just in case
                            return Value::Unknown;
                        };
                        let or_interval = self.eval(arg0);
                        return self_value.union(or_interval);
                    },
                    _ => {},
                }
            }
        }

        Value::full(ret_ty)
    }

    /// Uses the const eval machinery to evaluate an expression to a single
    /// integer value.
    fn const_eval(&self, expr: &Expr<'_>, ty: IntType) -> Value {
        if let Some(Constant::Int(n)) = self.const_eval.eval(expr) {
            return Self::u128_repr_to_interval(n, ty).or_full(ty);
        }
        Value::Unknown
    }

    fn is_same_variable(&self, expr: &Expr<'_>, other: &Expr<'_>) -> bool {
        // Check if the two expressions are the same variable
        if let ExprKind::Path(ref path) = expr.kind {
            if let ExprKind::Path(ref other_path) = other.kind {
                let res = self.cx.qpath_res(path, expr.hir_id);
                let other_res = self.cx.qpath_res(other_path, other.hir_id);
                return match (res, other_res) {
                    (Res::Local(lhs_id), Res::Local(rhs_id)) => lhs_id == rhs_id,
                    _ => false,
                };
            }
        }
        false
    }
    fn u128_repr_to_interval(n: u128, ty: IntType) -> Option<IInterval> {
        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let n = n as i128;
                // sign extend
                let amt = 128 - ty.bits() as u32;
                let n = (n << amt) >> amt;
                Some(IInterval::single_signed(ty, n))
            },
            IntTypeInfo::Unsigned(t_max) => {
                if n > t_max {
                    // this really shouldn't happen, but just in case
                    return None;
                }
                Some(IInterval::single_unsigned(ty, n))
            },
        }
    }
    pub fn to_int_type(&self, ty: Ty<'_>) -> Option<IntType> {
        match ty.kind() {
            TyKind::Int(IntTy::Isize) => Some(self.isize_ty),
            TyKind::Int(IntTy::I8) => Some(IntType::I8),
            TyKind::Int(IntTy::I16) => Some(IntType::I16),
            TyKind::Int(IntTy::I32) => Some(IntType::I32),
            TyKind::Int(IntTy::I64) => Some(IntType::I64),
            TyKind::Int(IntTy::I128) => Some(IntType::I128),
            TyKind::Uint(UintTy::Usize) => Some(self.isize_ty.to_unsigned()),
            TyKind::Uint(UintTy::U8) => Some(IntType::U8),
            TyKind::Uint(UintTy::U16) => Some(IntType::U16),
            TyKind::Uint(UintTy::U32) => Some(IntType::U32),
            TyKind::Uint(UintTy::U64) => Some(IntType::U64),
            TyKind::Uint(UintTy::U128) => Some(IntType::U128),
            _ => None,
        }
    }
}
