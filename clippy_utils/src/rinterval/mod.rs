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
use rustc_hir::{
    BinOpKind, Block, ConstBlock, Expr, ExprKind, HirId, Item, ItemKind, Node, PatExpr, PatExprKind, PathSegment,
    QPath, UnOp,
};
use rustc_lint::LateContext;
use rustc_middle::ty::{IntTy, Ty, TyCtxt, TyKind, TypeckResults, UintTy};
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
        }
    }

    /// Evaluates an expression to an integer interval.
    ///
    /// If the given expression is not of a supported integer type, None is
    /// returned.
    pub fn eval(&self, expr: &Expr<'cx>) -> Option<IInterval> {
        let ty = self.to_int_type(self.typeck.expr_ty(expr))?;

        let expr = self.cx.expr_or_init(expr.peel_borrows());

        if let Some(evaluated) = self.eval_ty(expr, ty) {
            Some(evaluated)
        } else {
            // we couldn't evaluate the expression for some reason, so just
            // return the full range of the integer type.
            Some(IInterval::full(ty))
        }
    }
    /// Evaluates an expression to an integer interval of the given type.
    ///
    /// If anything goes wrong or an expression cannot be evaluated, None is
    /// returned.
    fn eval_ty(&self, expr: &Expr<'cx>, ty: IntType) -> Option<IInterval> {
        match expr.kind {
            ExprKind::Lit(lit) => {
                return self.literal(&lit.node, ty);
            },
            ExprKind::Binary(op, lhs, rhs) => {
                return self.binary_op(op.node, lhs, rhs);
            },
            ExprKind::Unary(op, operand) => {
                let operand_interval = self.eval(operand)?;
                return self.unary_op(op, &operand_interval);
            },
            ExprKind::Cast(expr, _) => {
                let expr_interval = self.eval(expr)?;
                return Arithmetic::cast_as(&expr_interval, ty).ok();
            },

            // For conditional expressions, we evaluate all branches and
            // return the hull (union) of them.
            //
            // No attempt is made at trimming down on branches. All branches
            // are assumed to be reachable.
            ExprKind::If(_cond, if_true, Some(if_false)) => {
                let true_interval = self.eval(if_true)?;
                let false_interval = self.eval(if_false)?;
                return true_interval.hull(&false_interval);
            },
            ExprKind::Match(_expr, arms, _) => {
                let mut combined = IInterval::empty(ty);
                for arm in arms {
                    let arm_interval = self.eval(&arm.body)?;
                    combined = combined.hull(&arm_interval)?;
                }
                return Some(combined);
            },

            // Known methods and functions of integer types.
            ExprKind::MethodCall(path, self_arg, args, _) => return self.method_call(path, self_arg, args, ty),

            _ => {},
        }

        // if all else fails, try to evaluate the expression using const eval
        self.const_eval(expr, ty)
    }

    fn literal(&self, lit: &LitKind, ty: IntType) -> Option<IInterval> {
        match lit {
            LitKind::Int(n, _) => Self::u128_repr_to_interval(n.get(), ty),
            _ => None,
        }
    }
    fn binary_op(&self, op: BinOpKind, l_expr: &Expr<'cx>, r_expr: &Expr<'cx>) -> Option<IInterval> {
        let lhs = &self.eval(l_expr)?;

        // shl and shr have weird issues with type inference, so we need to
        // explicitly type the right-hand side as u32
        let rhs = if matches!(op, BinOpKind::Shl | BinOpKind::Shr) {
            &self.eval_ty(r_expr, IntType::U32).unwrap_or_else(|| {
                // if we can't evaluate the right-hand side, just return the full
                // range of the integer type.
                IInterval::full(IntType::U32)
            })
        } else {
            &self.eval(r_expr)?
        };

        match op {
            BinOpKind::Add => self.arth.add(lhs, rhs).ok(),
            BinOpKind::Sub => self.arth.sub(lhs, rhs).ok(),
            BinOpKind::Mul => self.arth.mul(lhs, rhs).ok(),
            BinOpKind::Div => self.arth.div(lhs, rhs).ok(),
            BinOpKind::Rem => self.arth.rem(lhs, rhs).ok(),
            BinOpKind::BitAnd => Arithmetic::and(lhs, rhs).ok(),
            BinOpKind::BitOr => Arithmetic::or(lhs, rhs).ok(),
            BinOpKind::BitXor => Arithmetic::xor(lhs, rhs).ok(),
            BinOpKind::Shl => self.arth.shl(lhs, rhs).ok(),
            BinOpKind::Shr => self.arth.shr(lhs, rhs).ok(),
            _ => None,
        }
    }
    fn unary_op(&self, op: UnOp, value: &IInterval) -> Option<IInterval> {
        match op {
            UnOp::Neg => self.arth.neg(value).ok(),
            UnOp::Not => Arithmetic::not(value).ok(),
            UnOp::Deref => {
                // Deref doesn't really make sense for numbers, but it does make
                // sense for references to numbers. Assuming that the value is
                // indeed a reference to a number, we can just return the value
                // of the number.
                Some(value.clone())
            },
        }
    }

    /// Calls to methods that returns an integer.
    fn method_call(
        &self,
        path: &PathSegment<'_>,
        self_arg: &Expr<'cx>,
        args: &[Expr<'cx>],
        ty: IntType,
    ) -> Option<IInterval> {
        match args {
            [] => {
                let f: Option<fn(&Arithmetic, &IInterval) -> ArithResult> = match path.ident.name {
                    sym::neg => Some(Arithmetic::neg),
                    sym::checked_neg => Some(|_, x| Arithmetic::strict_neg(x)),
                    sym::saturating_neg => Some(|_, x| Arithmetic::saturating_neg(x)),
                    sym::strict_neg => Some(|_, x| Arithmetic::strict_neg(x)),
                    sym::wrapping_neg => Some(|_, x| Arithmetic::wrapping_neg(x)),

                    sym::isqrt => Some(|_, x| Arithmetic::isqrt(x)),
                    sym::checked_isqrt => Some(|_, x| Arithmetic::isqrt(x)),

                    sym::abs => Some(Arithmetic::abs),
                    sym::checked_abs => Some(|_, x| Arithmetic::strict_abs(x)),
                    sym::saturating_abs => Some(|_, x| Arithmetic::saturating_abs(x)),
                    sym::strict_abs => Some(|_, x| Arithmetic::strict_abs(x)),
                    sym::wrapping_abs => Some(|_, x| Arithmetic::wrapping_abs(x)),
                    sym::unsigned_abs => Some(|_, x| Arithmetic::unsigned_abs(x)),

                    sym::not => Some(|_, x| Arithmetic::not(x)),

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

                if let Some(f) = f {
                    return f(&self.arth, &self.eval(self_arg)?).ok();
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

                    sym::pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::checked_pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::saturating_pow => Some(|_, l, r| Arithmetic::saturating_pow(l, r)),
                    sym::strict_pow => Some(|_, l, r| Arithmetic::strict_pow(l, r)),
                    sym::wrapping_pow => Some(|_, l, r| Arithmetic::wrapping_pow(l, r)),

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

                if let Some(f) = f {
                    return f(&self.arth, &self.eval(self_arg)?, &self.eval(arg1)?).ok();
                }
            },

            [arg1, arg2] => {
                let f: Option<fn(&Arithmetic, &IInterval, &IInterval, &IInterval) -> ArithResult> =
                    match path.ident.name {
                        sym::clamp => Some(|_, a, b, c| Arithmetic::max(b, &Arithmetic::min(a, c)?)),

                        _ => None,
                    };

                if let Some(f) = f {
                    return f(&self.arth, &self.eval(self_arg)?, &self.eval(arg1)?, &self.eval(arg2)?).ok();
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
                let self_interval = self.eval_ty(self_arg, ty)?;

                match path.ident.name {
                    sym::unwrap | sym::unwrap_unchecked | sym::expect => {
                        // these are all the same in that they return the Some value
                        return Some(self_interval);
                    },
                    sym::unwrap_or_default => {
                        // the default value of all integer types is 0, so we can
                        // evaluate the Some value and add 0 to it.
                        let zero = if ty.is_signed() {
                            IInterval::single_signed(ty, 0)
                        } else {
                            IInterval::single_unsigned(ty, 0)
                        };
                        return self_interval.hull(&zero);
                    },
                    sym::unwrap_or => {
                        // the default value is given as the second argument
                        let or_interval = self.eval(args.get(0)?)?;
                        return self_interval.hull(&or_interval);
                    },
                    _ => {},
                }
            }
        }

        None
    }

    /// Uses the const eval machinery to evaluate an expression to a single
    /// integer value.
    fn const_eval(&self, expr: &Expr<'_>, ty: IntType) -> Option<IInterval> {
        let const_val = self.const_eval.eval(expr)?;
        if let Constant::Int(n) = const_val {
            return Self::u128_repr_to_interval(n, ty);
        }
        None
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
    fn to_int_type(&self, ty: Ty<'_>) -> Option<IntType> {
        match ty.kind() {
            TyKind::Int(IntTy::Isize) => Some(self.isize_ty),
            TyKind::Int(IntTy::I8) => Some(IntType::I8),
            TyKind::Int(IntTy::I16) => Some(IntType::I16),
            TyKind::Int(IntTy::I32) => Some(IntType::I32),
            TyKind::Int(IntTy::I64) => Some(IntType::I64),
            TyKind::Int(IntTy::I128) => Some(IntType::I128),
            TyKind::Uint(UintTy::Usize) => Some(self.isize_ty.swap_signedness()),
            TyKind::Uint(UintTy::U8) => Some(IntType::U8),
            TyKind::Uint(UintTy::U16) => Some(IntType::U16),
            TyKind::Uint(UintTy::U32) => Some(IntType::U32),
            TyKind::Uint(UintTy::U64) => Some(IntType::U64),
            TyKind::Uint(UintTy::U128) => Some(IntType::U128),
            _ => None,
        }
    }
}
