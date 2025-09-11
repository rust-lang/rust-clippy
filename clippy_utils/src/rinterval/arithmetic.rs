use super::bits::Bits;
use super::{IInterval, IntType, IntTypeInfo};

#[derive(Debug)]
pub enum ArithError {
    TypeError,
    Unsupported,
}

pub type ArithResult<T = IInterval> = Result<T, ArithError>;

fn check_same_ty(lhs: &IInterval, rhs: &IInterval) -> ArithResult<IntType> {
    if lhs.ty != rhs.ty {
        return Err(ArithError::TypeError);
    }
    Ok(lhs.ty)
}

macro_rules! check_non_empty {
    ($x:expr) => {
        if $x.is_empty() {
            return Ok(IInterval::empty($x.ty));
        }
    };
    ($lhs:expr, $rhs:expr) => {
        if $lhs.is_empty() || $rhs.is_empty() {
            return Ok(IInterval::empty($lhs.ty));
        }
    };
}

#[derive(Clone, Copy, PartialEq)]
enum Overflow {
    None,
    Under,
    Over,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignBit {
    NonNeg = 1,
    Neg = -1,
}

fn min_4(values: &[i128; 4]) -> i128 {
    values[0].min(values[1]).min(values[2]).min(values[3])
}
fn max_4(values: &[i128; 4]) -> i128 {
    values[0].max(values[1]).max(values[2]).max(values[3])
}
fn range_4(ty: IntType, values: [i128; 4]) -> IInterval {
    debug_assert!(ty.is_signed());
    IInterval::new_signed(ty, min_4(&values), max_4(&values))
}

/// Splits the interval by the sign bit of its values. The given function will
/// called with the min and max values of unsigned intervals.
///
/// E.g. `f` will be called with `(1, 10)` for the interval `[1, 10]` and with
/// `(0, 5), (u128::MAX-4, u128::MAX)` for the interval `[-5, 5]`.
fn split_by_sign_bit(i: &IInterval, mut f: impl FnMut(u128, u128) -> IInterval) -> IInterval {
    debug_assert!(!i.is_empty());

    if i.ty.is_signed() {
        let (min, max) = i.as_signed();

        if min < 0 {
            if max >= 0 {
                f(min.cast_unsigned(), u128::MAX).hull_unwrap(&f(min.max(0).cast_unsigned(), max.cast_unsigned()))
            } else {
                f(min.cast_unsigned(), max.cast_unsigned())
            }
        } else {
            f(min.cast_unsigned(), max.cast_unsigned())
        }
    } else {
        let (min, max) = i.as_unsigned();
        f(min, max)
    }
}
/// Same as `split_by_sign_bit`, but only for signed intervals.
fn split_by_sign_bit_signed(i: &IInterval, mut f: impl FnMut(i128, i128, SignBit) -> IInterval) -> IInterval {
    debug_assert!(!i.is_empty());
    debug_assert!(i.ty.is_signed());

    let (min, max) = i.as_signed();

    if min < 0 {
        if max >= 0 {
            f(min, -1, SignBit::Neg).hull_unwrap(&f(min.max(0), max, SignBit::NonNeg))
        } else {
            f(min, max, SignBit::Neg)
        }
    } else {
        f(min, max, SignBit::NonNeg)
    }
}

fn parse_shift_strict(shift: &IInterval, bit_width: u8) -> Option<(u8, u8)> {
    if shift.is_empty() {
        return None;
    }

    if shift.ty.is_signed() {
        let (min, max) = shift.as_signed();
        if max < 0 || min >= bit_width as i128 {
            return None;
        }

        Some((min.max(0) as u8, max.min((bit_width - 1) as i128) as u8))
    } else {
        let (min, max) = shift.as_unsigned();
        if min >= bit_width as u128 {
            return None;
        }

        Some((min as u8, max.min((bit_width - 1) as u128) as u8))
    }
}
fn parse_shift_wrapping(shift: &IInterval, bit_width: u8) -> Option<(u8, u8)> {
    if shift.is_empty() {
        return None;
    }

    // check for large ranges
    if shift.ty.is_signed() {
        let (min, max) = shift.as_signed();
        if max.abs_diff(min) >= (bit_width - 1) as u128 {
            return Some((0, bit_width - 1));
        }
    } else {
        let (min, max) = shift.as_unsigned();
        if max - min >= (bit_width - 1) as u128 {
            return Some((0, bit_width - 1));
        }
    }

    // due to how the `% bit_width`, we can completely ignore the maybe sign of
    // the values and just cast to u8.
    let min = (shift.min as u8) % bit_width;
    let max = (shift.max as u8) % bit_width;

    Some((min, max))
}

pub struct Arithmetic {
    /// If `true`, checked arithmetic will be assumed.
    ///
    /// Suppose we have an expression `x + y` and we know that `x` and `y` are
    /// `u8`s with the ranges `0 <= x <= 200` and `100 <= y <= 200`. If not for
    /// the limited bit width of `u8`, the expression `x + y` *would* have a
    /// range `100 <= x + y <= 400`. However, since `u8` can only hold values
    /// up to 255, so overflow occurs.
    ///
    /// If checked arithmetic is assumed, then the range of the expression is
    /// `100 <= x + y <= 255`. Since the addition will panic on overflow, no
    /// other numbers can be produced.
    ///
    /// If unchecked arithmetic is assumed, then the range of the expression is
    /// `0 <= x + y <= 255`. Since addition will wrap on overflow, both 0 and
    /// 255 are possible results.
    pub checked: bool,
}

impl Arithmetic {
    pub fn add(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_add(left, right)
        } else {
            Self::wrapping_add(left, right)
        }
    }
    pub fn neg(&self, value: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_neg(value)
        } else {
            Self::wrapping_neg(value)
        }
    }
    pub fn sub(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_sub(left, right)
        } else {
            Self::wrapping_sub(left, right)
        }
    }
    pub fn mul(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_mul(left, right)
        } else {
            Self::wrapping_mul(left, right)
        }
    }
    pub fn div(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_div(left, right)
        } else {
            Self::wrapping_div(left, right)
        }
    }
    pub fn rem(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_rem(left, right)
        } else {
            Self::wrapping_rem(left, right)
        }
    }
    pub fn rem_euclid(&self, left: &IInterval, right: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_rem_euclid(left, right)
        } else {
            Self::wrapping_rem_euclid(left, right)
        }
    }
    pub fn abs(&self, value: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_abs(value)
        } else {
            Self::wrapping_abs(value)
        }
    }
    pub fn shl(&self, value: &IInterval, shift: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_shl(value, shift)
        } else {
            Self::wrapping_shl(value, shift)
        }
    }
    pub fn shr(&self, value: &IInterval, shift: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_shr(value, shift)
        } else {
            Self::wrapping_shr(value, shift)
        }
    }
    pub fn next_power_of_two(&self, value: &IInterval) -> ArithResult {
        if self.checked {
            Self::strict_next_power_of_two(value)
        } else {
            Self::wrapping_next_power_of_two(value)
        }
    }

    /// Addition which saturates on overflow.
    pub fn saturating_add(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = l_min.saturating_add(r_min).clamp(t_min, t_max);
                let max = l_max.saturating_add(r_max).clamp(t_min, t_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.saturating_add(r_min).min(t_max);
                let max = l_max.saturating_add(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Addition which panics on overflow.
    pub fn strict_add(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = if l_min < 0 {
                    // only underflow is possible, so saturate
                    l_min.saturating_add(r_min).max(t_min)
                } else {
                    // only overflow is possible
                    let Some(min) = l_min.checked_add(r_min) else {
                        // the sum will always overflow
                        return Ok(IInterval::empty(ty));
                    };
                    if min > t_max {
                        // the sum will always overflow
                        return Ok(IInterval::empty(ty));
                    }
                    min
                };

                let max = if l_max < 0 {
                    // only underflow is possible
                    let Some(max) = l_max.checked_add(r_max) else {
                        // the sum will always underflow
                        return Ok(IInterval::empty(ty));
                    };
                    if max < t_min {
                        // the sum will always underflow
                        return Ok(IInterval::empty(ty));
                    }
                    max
                } else {
                    // only overflow is possible, so saturate
                    l_max.saturating_add(r_max).min(t_max)
                };

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let Some(min) = l_min.checked_add(r_min) else {
                    // the sum will always overflow
                    return Ok(IInterval::empty(ty));
                };
                if min > t_max {
                    // the sum will always overflow
                    return Ok(IInterval::empty(ty));
                }
                let max = l_max.saturating_add(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Addition which wraps on overflow.
    pub fn wrapping_add(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let (mut min, min_overflow) = l_min.overflowing_add(r_min);
                let (mut max, max_overflow) = l_max.overflowing_add(r_max);

                let min_overflow = if min_overflow {
                    if l_min < 0 { Overflow::Under } else { Overflow::Over }
                } else if min < t_min {
                    min -= t_min * 2;
                    Overflow::Under
                } else if min > t_max {
                    min += t_min * 2;
                    Overflow::Over
                } else {
                    Overflow::None
                };
                let max_overflow = if max_overflow {
                    if l_max < 0 { Overflow::Under } else { Overflow::Over }
                } else if max < t_min {
                    max -= t_min * 2;
                    Overflow::Under
                } else if max > t_max {
                    max += t_min * 2;
                    Overflow::Over
                } else {
                    Overflow::None
                };

                if min_overflow == max_overflow {
                    // If both overflow the same way, the result is simply the range
                    Ok(IInterval::new_signed(ty, min, max))
                } else if min_overflow == Overflow::None || max_overflow == Overflow::None {
                    // If one doesn't over/underflow while the other does,
                    // then the result is the entire range.
                    Ok(IInterval::new_signed(ty, t_min, t_max))
                } else {
                    // Lastly, min underflow while max overflows.
                    // Idk what to do in this case, so just return the entire range.
                    Ok(IInterval::new_signed(ty, t_min, t_max))
                }
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let (mut min, mut min_overflow) = l_min.overflowing_add(r_min);
                let (mut max, mut max_overflow) = l_max.overflowing_add(r_max);

                if min > t_max {
                    min &= t_max;
                    min_overflow = true;
                }
                if max > t_max {
                    max &= t_max;
                    max_overflow = true;
                }

                if !min_overflow && max_overflow {
                    // this means that both 0 and t_max are possible results
                    return Ok(IInterval::new_unsigned(ty, 0, t_max));
                }

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Negation which saturates on overflow.
    pub fn saturating_neg(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (x_min, x_max) = x.as_signed();

                let min = x_max.saturating_neg().min(t_max);
                let max = x_min.saturating_neg().min(t_max);

                Ok(IInterval::new_signed(x.ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }
    /// Negation which panics on overflow.
    pub fn strict_neg(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (mut x_min, x_max) = x.as_signed();

                if x_max == t_min {
                    // all values in the range will overflow
                    Ok(IInterval::empty(x.ty))
                } else {
                    if x_min == t_min {
                        x_min += 1; // ignore value that will overflow
                    }

                    Ok(IInterval::new_signed(x.ty, -x_max, -x_min))
                }
            },
            IntTypeInfo::Unsigned(_) => {
                let (x_min, _) = x.as_unsigned();

                if x_min == 0 {
                    // contains zero
                    Ok(IInterval::new_unsigned(x.ty, 0, 0))
                } else {
                    Ok(IInterval::empty(x.ty))
                }
            },
        }
    }
    /// Negation which wraps on overflow.
    pub fn wrapping_neg(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (mut x_min, x_max) = x.as_signed();

                if x_max == t_min {
                    // all values in the range will overflow
                    Ok(x.clone())
                } else {
                    let overflow = x_min == t_min;
                    if overflow {
                        x_min += 1; // ignore value that will overflow
                    }

                    let min = if overflow { t_min } else { -x_max };
                    let max = -x_min;

                    Ok(IInterval::new_signed(x.ty, min, max))
                }
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (x_min, x_max) = x.as_unsigned();

                if x_min == 0 && x_max != 0 {
                    // this means that the range wraps around and covers both 0
                    Ok(IInterval::new_unsigned(x.ty, 0, t_max))
                } else {
                    let min = x_max.wrapping_neg() & t_max;
                    let max = x_min.wrapping_neg() & t_max;

                    Ok(IInterval::new_unsigned(x.ty, min, max))
                }
            },
        }
    }

    /// Subtraction which saturates on overflow.
    pub fn saturating_sub(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = l_min.saturating_sub(r_max).clamp(t_min, t_max);
                let max = l_max.saturating_sub(r_min).clamp(t_min, t_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.saturating_sub(r_max);
                let max = l_max.saturating_sub(r_min);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Subtraction which panics on overflow.
    pub fn strict_sub(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (l_min, l_max) = lhs.as_signed();
                let (mut r_min, r_max) = rhs.as_signed();

                // The idea here is to calculate `lhs - rhs` as `lhs + rhs.neg()`.
                // This doesn't work for rhs == t_min, because negating it will overflow,
                // so we have to handle that case separately.

                let min_range = if r_min == t_min {
                    // range for `lhs - t_min`
                    let min_range = if l_min >= 0 {
                        // lhs >= 0, so the result will always overflow
                        IInterval::empty(ty)
                    } else {
                        // lhs < 0, so the result will always underflow
                        IInterval::new_signed(ty, l_min - t_min, l_max.saturating_sub(t_min).min(t_max))
                    };

                    r_min += 1;

                    if r_max == t_min {
                        return Ok(min_range);
                    }

                    min_range
                } else {
                    IInterval::empty(ty)
                };

                // we can now safely negate rhs
                let rhs_neg = IInterval::new_signed(ty, -r_max, -r_min);
                let sum = Self::strict_add(lhs, &rhs_neg)?;
                Ok(sum.hull_unwrap(&min_range))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.saturating_sub(r_max);
                let (max, overflows) = l_max.overflowing_sub(r_min);

                if overflows {
                    Ok(IInterval::empty(ty))
                } else {
                    Ok(IInterval::new_unsigned(ty, min, max))
                }
            },
        }
    }
    /// Subtraction which wrap on overflow.
    pub fn wrapping_sub(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        Self::wrapping_add(lhs, &Self::wrapping_neg(rhs)?)
    }

    /// Multiplication which saturates on overflow and panics on rhs == 0.
    pub fn saturating_mul(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let points = [
                    l_min.saturating_mul(r_min),
                    l_min.saturating_mul(r_max),
                    l_max.saturating_mul(r_min),
                    l_max.saturating_mul(r_max),
                ];
                let min = min_4(&points).clamp(t_min, t_max);
                let max = max_4(&points).clamp(t_min, t_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.saturating_mul(r_min).min(t_max);
                let max = l_max.saturating_mul(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Multiplication which panics on overflow and panics on rhs == 0.
    pub fn strict_mul(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let quadrant = |mut l_min: i128,
                                mut l_max: i128,
                                mut l_sign: SignBit,
                                mut r_min: i128,
                                mut r_max: i128,
                                mut r_sign: SignBit|
                 -> IInterval {
                    debug_assert!(l_min > 0 || l_max < 0);
                    debug_assert!(r_min > 0 || r_max < 0);

                    if l_sign == SignBit::NonNeg && r_sign == SignBit::Neg {
                        std::mem::swap(&mut l_min, &mut r_min);
                        std::mem::swap(&mut l_max, &mut r_max);
                        std::mem::swap(&mut l_sign, &mut r_sign);
                    }

                    match (l_sign, r_sign) {
                        (SignBit::NonNeg, SignBit::NonNeg) => {
                            // both positive
                            let (min, min_overflow) = l_min.overflowing_mul(r_min);
                            if min_overflow || min > t_max {
                                // the multiplication will always overflow
                                return IInterval::empty(ty);
                            }
                            IInterval::new_signed(ty, min, l_max.saturating_mul(r_max).min(t_max))
                        },
                        (SignBit::NonNeg, SignBit::Neg) => unreachable!(),
                        (SignBit::Neg, SignBit::NonNeg) => {
                            // lhs negative, rhs positive
                            // both positive
                            let (max, max_overflow) = l_max.overflowing_mul(r_min);
                            if max_overflow || max < t_min {
                                // the multiplication will always overflow
                                return IInterval::empty(ty);
                            }
                            IInterval::new_signed(ty, l_min.saturating_mul(r_max).max(t_min), max)
                        },
                        (SignBit::Neg, SignBit::Neg) => {
                            // both negative
                            let (min, min_overflow) = l_max.overflowing_mul(r_max);
                            if min_overflow || min > t_max {
                                // the multiplication will always overflow
                                return IInterval::empty(ty);
                            }
                            IInterval::new_signed(ty, l_max * r_max, l_min.saturating_mul(r_min).min(t_max))
                        },
                    }
                };

                let split_l = |r_min: i128, r_max: i128, r_sign: SignBit| -> IInterval {
                    debug_assert!(r_min > 0 || r_max < 0);

                    let mut result = IInterval::empty(ty);

                    if l_min < 0 {
                        result =
                            result.hull_unwrap(&quadrant(l_min, l_max.min(-1), SignBit::Neg, r_min, r_max, r_sign));
                    }
                    if l_min <= 0 && 0 <= l_max {
                        result = result.hull_unwrap(&IInterval::single_signed(ty, 0));
                    }
                    if l_max > 0 {
                        result =
                            result.hull_unwrap(&quadrant(l_min.max(1), l_max, SignBit::NonNeg, r_min, r_max, r_sign));
                    }

                    result
                };

                let mut result = IInterval::empty(ty);

                if r_min < 0 {
                    result = result.hull_unwrap(&split_l(r_min, r_max.min(-1), SignBit::Neg));
                }
                if r_min <= 0 && 0 <= r_max {
                    result = result.hull_unwrap(&IInterval::single_signed(ty, 0));
                }
                if r_max > 0 {
                    result = result.hull_unwrap(&split_l(r_min.max(1), r_max, SignBit::NonNeg));
                }

                Ok(result)
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let (min, min_overflow) = l_min.overflowing_mul(r_min);
                if min_overflow || min > t_max {
                    // the multiplication will always overflow
                    return Ok(IInterval::empty(ty));
                }
                let max = l_max.saturating_mul(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Multiplication which wraps on overflow and panics on rhs == 0.
    pub fn wrapping_mul(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let (p0, p0_overflow) = l_min.overflowing_mul(r_min);
                let (p1, p1_overflow) = l_min.overflowing_mul(r_max);
                let (p2, p2_overflow) = l_max.overflowing_mul(r_min);
                let (p3, p3_overflow) = l_max.overflowing_mul(r_max);

                if !p0_overflow && !p1_overflow && !p2_overflow && !p3_overflow {
                    let points = [p0, p1, p2, p3];
                    let min = min_4(&points);
                    let max = max_4(&points);
                    debug_assert!(min <= max);
                    if t_min <= min && max <= t_max {
                        return Ok(IInterval::new_signed(ty, min, max));
                    }
                }

                Ok(IInterval::full(ty))
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let mul_single = |l_min: u128, l_max: u128, r: u128| -> IInterval {
                    let min = l_min.wrapping_mul(r) & t_max;
                    let max = l_max.wrapping_mul(r) & t_max;
                    if min <= max && (l_max - l_min).saturating_mul(r) < t_max {
                        IInterval::new_unsigned(ty, min, max)
                    } else {
                        IInterval::full(ty)
                    }
                };

                let (max, max_overflow) = l_max.overflowing_mul(r_max);
                if max_overflow || max > t_max {
                    let range = if l_min == l_max {
                        mul_single(r_min, r_max, l_min)
                    } else if r_min == r_max {
                        mul_single(l_min, l_max, r_min)
                    } else {
                        // I'm out of ideas
                        IInterval::full(ty)
                    };
                    return Ok(range);
                }
                let min = l_min.wrapping_mul(r_min);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Division which saturates on overflow and panics on rhs == 0.
    pub fn saturating_div(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                // the only difference between saturating_div and strict_div is
                // the case t_min / -1, because it's the only case which overflows

                let strict = Self::strict_div(lhs, rhs)?;

                let (l_min, _) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                if l_min == t_min && r_min <= -1 && -1 <= r_max {
                    // t_min / -1 will overflow, so we have to add t_min to the result
                    Ok(IInterval::single_signed(ty, t_max).hull_unwrap(&strict))
                } else {
                    Ok(strict)
                }
            },
            // same as strict_div for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_div(lhs, rhs),
        }
    }
    /// Division which panics on overflow and rhs == 0.
    pub fn strict_div(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                // We have to split the rhs into 4 cases:
                // 1. -inf..=-2: the negative range
                // 2. -1: t_min / -1 overflows, which has to be handled separately
                // 3. 0: division by zero panics
                // 4. 1..=inf: the positive range

                // this will be the total union of all cases
                let mut result = IInterval::empty(ty);

                // case 1: -inf..=-2
                if r_min <= -2 {
                    let r_max = r_max.min(-2);

                    let points = [l_min / r_min, l_min / r_max, l_max / r_min, l_max / r_max];
                    result = result.hull_unwrap(&range_4(ty, points));
                }

                // case 2: -1
                if r_min <= -1 && -1 <= r_max {
                    // same as strict_neg
                    result = result.hull_unwrap(&Self::strict_neg(lhs)?);
                }

                // case 3: 0
                // This will always panic, so it doesn't contribute to the result.

                // case 4: 1..=inf
                if r_max >= 1 {
                    let r_min = r_min.max(1);

                    let points = [l_min / r_min, l_min / r_max, l_max / r_min, l_max / r_max];
                    result = result.hull_unwrap(&range_4(ty, points));
                }

                Ok(result)
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (mut r_min, r_max) = rhs.as_unsigned();

                if r_max == 0 {
                    // always div by 0
                    return Ok(IInterval::empty(ty));
                }
                if r_min == 0 {
                    r_min = 1; // to avoid division by zero
                }

                let min = l_min / r_max;
                let max = l_max / r_min;

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Division which wrap on overflow and panics on rhs == 0.
    pub fn wrapping_div(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, _) => {
                // the only difference between wrapping_div and strict_div is
                // the case t_min / -1, because it's the only case which overflows

                let strict = Self::strict_div(lhs, rhs)?;

                let (l_min, _) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                if l_min == t_min && r_min <= -1 && -1 <= r_max {
                    // t_min / -1 will overflow, so we have to add t_min to the result
                    Ok(IInterval::single_signed(ty, t_min).hull_unwrap(&strict))
                } else {
                    Ok(strict)
                }
            },
            // same as strict_div for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_div(lhs, rhs),
        }
    }

    /// Division which panics on overflow and rhs == 0.
    pub fn strict_div_euclid(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                // TODO: implement this properly

                let (l_min, _) = lhs.as_signed();
                let (r_min, _) = rhs.as_signed();

                if l_min >= 0 && r_min >= 0 {
                    // both positive
                    return Self::strict_div(lhs, rhs);
                }

                Ok(IInterval::full(ty))
            },
            IntTypeInfo::Unsigned(_) => Self::strict_div(lhs, rhs),
        }
    }
    /// Division which wrap on overflow and panics on rhs == 0.
    pub fn wrapping_div_euclid(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, _) => {
                // the only difference between wrapping_div_euclid and
                // strict_div_euclid is the case t_min / -1

                let strict = Self::strict_div_euclid(lhs, rhs)?;

                let (l_min, _) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                if l_min == t_min && r_min <= -1 && -1 <= r_max {
                    // t_min / -1 will overflow, so we have to add t_min to the result
                    Ok(IInterval::single_signed(ty, t_min).hull_unwrap(&strict))
                } else {
                    Ok(strict)
                }
            },
            // same as strict_div for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_div(lhs, rhs),
        }
    }

    /// Division which rounds towards positive infinity and panics on rhs == 0.
    pub fn div_ceil(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(_, _) => Err(ArithError::Unsupported),
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (mut r_min, r_max) = rhs.as_unsigned();

                if r_max == 0 {
                    // always div by 0
                    return Ok(IInterval::empty(ty));
                }
                if r_min == 0 {
                    r_min = 1;
                }

                let min = l_min.div_ceil(r_max);
                let max = l_max.div_ceil(r_min);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Remainder which panics on overflow and rhs == 0.
    pub fn strict_rem(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (l_min, l_max) = lhs.as_signed();
                let (mut r_min, r_max) = rhs.as_signed();

                // Okay, so remainder is a pain to implement, because the
                // operation works as follows:
                // 1. If rhs == 0, panic.
                // 2. If rhs == -1 and lhs == t_min, panic.
                // 3. If rhs < 0, return lhs % -rhs.
                // 4. If lhs < 0, return -(-lhs % rhs).
                // 5. Return lhs % rhs (everything unsigned).
                // Note that -rhs and -lhs can overflow , so that needs
                // to be handled separately too.

                let mut result = IInterval::empty(ty);

                // handle rhs == t_min separately
                if r_min == t_min {
                    let min_range = if l_min == t_min {
                        let zero = IInterval::single_signed(ty, 0);
                        if l_max == t_min {
                            zero
                        } else {
                            zero.hull_unwrap(&IInterval::new_signed(ty, l_min + 1, l_max))
                        }
                    } else {
                        lhs.clone()
                    };

                    if r_max == t_min {
                        return Ok(min_range);
                    }

                    result = min_range;
                    r_min += 1;
                    // this case is handled now, which means we can now safely
                    // compute -r_min and -r_max
                }

                let positive_everything = |l_min: i128, l_max: i128, r_min: i128, r_max: i128| -> IInterval {
                    debug_assert!(0 <= l_min && l_min <= l_max && l_max <= t_max);
                    debug_assert!(0 <= r_min && r_min <= r_max && r_max <= t_max);

                    // if the rhs is a single value, this is possible
                    if r_min == r_max {
                        let r = r_min;
                        // if the lhs as more or equal values than the rhs, then the
                        // result is the trivial range [0, r - 1], which isn't
                        // interesting
                        if l_max - l_min < r {
                            let min = l_min % r;
                            let max = l_max % r;
                            if min <= max {
                                return IInterval::new_signed(ty, min, max);
                            }
                        }
                    }

                    if l_max < r_min {
                        return IInterval::new_signed(ty, l_min, l_max);
                    }

                    IInterval::new_signed(ty, 0, l_max.min(r_max - 1))
                };
                let positive_rhs = |r_min: i128, r_max: i128| -> IInterval {
                    debug_assert!(0 < r_min && r_min <= r_max && r_max <= t_max);

                    let mut l_min = l_min;

                    let min_range = if l_min == t_min {
                        l_min += 1;
                        let min_range = if r_min == r_max {
                            IInterval::single_signed(ty, t_min % r_min)
                        } else {
                            IInterval::new_signed(ty, -r_max + 1, 0)
                        };

                        if l_max == t_min {
                            return min_range;
                        }

                        min_range
                    } else {
                        IInterval::empty(ty)
                    };

                    let negative = if l_min < 0 {
                        // this is -(-lhs & rhs)
                        let (min, max) = positive_everything((-l_max).max(0), -l_min, r_min, r_max).as_signed();
                        IInterval::new_signed(ty, -max, -min)
                    } else {
                        IInterval::empty(ty)
                    };
                    let positive = if l_max >= 0 {
                        positive_everything(l_min.max(0), l_max, r_min, r_max)
                    } else {
                        IInterval::empty(ty)
                    };

                    negative.hull_unwrap(&positive).hull_unwrap(&min_range)
                };

                // case 1: -inf..=-2
                if r_min <= -2 {
                    result = result.hull_unwrap(&positive_rhs(-r_max.min(-2), -r_min));
                }

                // case 2: -1
                if r_min <= -1 && -1 <= r_max {
                    // t_min % -1 panics, while everything else goes to 0
                    if l_max != t_min {
                        result = result.hull_unwrap(&IInterval::single_signed(ty, 0));
                    }
                }

                // case 3: 0
                // This will always panic, so it doesn't contribute to the result.

                // case 4: 1..=inf
                if r_max >= 1 {
                    result = result.hull_unwrap(&positive_rhs(r_min.max(1), r_max));
                }

                Ok(result)
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (mut r_min, r_max) = rhs.as_unsigned();

                if r_max == 0 {
                    // always div by 0
                    return Ok(IInterval::empty(ty));
                }
                if r_min == 0 {
                    r_min = 1; // to avoid division by zero
                }

                // if the rhs is a single value, this is possible
                if r_min == r_max {
                    let r = r_min;
                    // if the lhs as more or equal values than the rhs, then the
                    // result is the trivial range [0, r - 1], which isn't
                    // interesting
                    if l_max - l_min < r {
                        let min = l_min % r;
                        let max = l_max % r;
                        if min <= max {
                            return Ok(IInterval::new_unsigned(ty, min, max));
                        }
                    }
                }

                if l_max < r_min {
                    return Ok(lhs.clone());
                }

                Ok(IInterval::new_unsigned(ty, 0, l_max.min(r_max - 1)))
            },
        }
    }
    /// Remainder which wrap on overflow and panics on rhs == 0.
    pub fn wrapping_rem(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, _) => {
                // the only difference between wrapping_rem and strict_rem is
                // the case t_min % -1

                let strict = Self::strict_rem(lhs, rhs)?;

                let (l_min, _) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                if l_min == t_min && r_min <= -1 && -1 <= r_max {
                    // t_min % -1 == 0 when wrapping
                    Ok(IInterval::single_signed(ty, 0).hull_unwrap(&strict))
                } else {
                    Ok(strict)
                }
            },
            // same as strict_div for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_rem(lhs, rhs),
        }
    }

    /// Modulo which panics on overflow and rhs == 0.
    pub fn strict_rem_euclid(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (mut r_min, mut r_max) = rhs.as_signed();

                // Okay, so modulo is a pain to implement, because the
                // operation works as follows:
                // 1. If rhs == 0, panic.
                // 2. If rhs == -1 and lhs == t_min, panic.
                // 3. Return lhs mod abs(rhs)
                // Note that abs(rhs) can overflow.

                let mut result = IInterval::empty(ty);

                // handle rhs == t_min separately
                if r_min == t_min {
                    let min_range = split_by_sign_bit_signed(lhs, |min, max, sign| {
                        if sign == SignBit::Neg {
                            IInterval::new_signed(ty, min - t_min, max - t_min)
                        } else {
                            IInterval::new_signed(ty, min, max)
                        }
                    });

                    if r_max == t_min {
                        return Ok(min_range);
                    }

                    result = min_range;
                    r_min += 1;
                    // this case is handled now, which means we can now safely
                    // compute -r_min and -r_max
                }

                debug_assert!(
                    r_min <= r_max && r_max <= t_max,
                    "Invalid rhs: [{r_min}, {r_max}] for type {ty:?}"
                );

                // Very annoyingly, t_min (mod -1) panics. Since all operations
                // only guarantee to result a superset, I will just ignore this
                // panic and pretend that lhs (mod -1) == lhs (mod 1).
                // With that out of the way, calculate abs(rhs)
                if r_max < 0 {
                    (r_min, r_max) = (-r_max, -r_min);
                } else if r_min < 0 {
                    (r_min, r_max) = (0, r_max.max(-r_min));
                }

                if r_min == 0 {
                    // rhs=0 always panics, so ignore it
                    if r_max == 0 {
                        debug_assert!(result.is_empty());
                        return Ok(IInterval::empty(ty));
                    }
                    r_min = 1; // to avoid division by zero
                }

                debug_assert!(
                    0 < r_min && r_min <= r_max && r_max <= t_max,
                    "Invalid rhs: [{r_min}, {r_max}] for type {ty:?}"
                );

                // now the general case, aka the bulk of the function
                let general = split_by_sign_bit_signed(lhs, |l_min, l_max, sign| {
                    if r_min == r_max {
                        // if the rhs is a single value, this is possible
                        // if the lhs as more or equal values than the rhs, then the
                        // result is the trivial range [0, r - 1], which isn't
                        // interesting
                        if l_max - l_min < r_min {
                            let min = l_min.rem_euclid(r_min);
                            let max = l_max.rem_euclid(r_min);
                            if min <= max {
                                return IInterval::new_signed(ty, min, max);
                            }
                        }
                        return IInterval::new_signed(ty, 0, r_max - 1);
                    }

                    if sign == SignBit::NonNeg && l_max < r_min {
                        // Since 0 <= lhs <= rhs, lhs (mod rhs) == lhs
                        return IInterval::new_signed(ty, l_min, l_max);
                    }

                    if sign == SignBit::NonNeg {
                        return IInterval::new_signed(ty, 0, l_max.min(r_max - 1));
                    }

                    IInterval::new_signed(ty, 0, r_max - 1)
                });

                Ok(result.hull_unwrap(&general))
            },
            // same as strict_rem for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_rem(lhs, rhs),
        }
    }
    /// Modulo which wrap on overflow and panics on rhs == 0.
    pub fn wrapping_rem_euclid(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(t_min, _) => {
                // the only difference between wrapping_rem and strict_rem is
                // the case t_min % -1

                let strict = Self::strict_rem_euclid(lhs, rhs)?;

                let (l_min, _) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                if l_min == t_min && r_min <= -1 && -1 <= r_max {
                    // t_min % -1 == 0 when wrapping
                    Ok(IInterval::single_signed(ty, 0).hull_unwrap(&strict))
                } else {
                    Ok(strict)
                }
            },
            // same as strict_rem for unsigned types
            IntTypeInfo::Unsigned(_) => Self::strict_rem(lhs, rhs),
        }
    }

    /// Midpoint.
    pub fn midpoint(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = l_min.midpoint(r_min);
                let max = l_max.midpoint(r_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.midpoint(r_min);
                let max = l_max.midpoint(r_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Integer square root, which panics for negative values.
    pub fn isqrt(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        let ty = x.ty;

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (mut x_min, x_max) = x.as_signed();
                if x_max < 0 {
                    return Ok(IInterval::empty(ty));
                }
                if x_min < 0 {
                    x_min = 0; // ignore negative values
                }

                let min = x_min.isqrt();
                let max = x_max.isqrt();

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (x_min, x_max) = x.as_unsigned();

                let min = x_min.isqrt();
                let max = x_max.isqrt();

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Log2, which panics for values <= 0.
    pub fn ilog(x: &IInterval, base: &IInterval) -> ArithResult {
        let ty = check_same_ty(x, base)?;

        if x.is_empty() || base.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let (min, max) = match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (mut x_min, x_max) = x.as_signed();
                let (mut base_min, base_max) = base.as_signed();

                if x_max <= 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min <= 0 {
                    x_min = 1; // ignore non-positive values
                }

                if base_max < 2 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if base_min < 2 {
                    base_min = 2;
                }

                (x_min.ilog(base_max), x_max.ilog(base_min))
            },
            IntTypeInfo::Unsigned(_) => {
                let (mut x_min, x_max) = x.as_unsigned();
                let (mut base_min, base_max) = base.as_unsigned();

                if x_max == 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min == 0 {
                    x_min = 1; // ignore non-positive values
                }

                if base_max < 2 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if base_min < 2 {
                    base_min = 2;
                }

                (x_min.ilog(base_max), x_max.ilog(base_min))
            },
        };

        Ok(IInterval::new_unsigned(IntType::U32, min as u128, max as u128))
    }
    /// Log2, which panics for values <= 0.
    pub fn ilog2(x: &IInterval) -> ArithResult {
        if x.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let ty = x.ty;

        let (min, max) = match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (mut x_min, x_max) = x.as_signed();
                if x_max <= 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min <= 0 {
                    x_min = 1; // ignore non-positive values
                }

                (x_min.ilog2(), x_max.ilog2())
            },
            IntTypeInfo::Unsigned(_) => {
                let (mut x_min, x_max) = x.as_unsigned();
                if x_max == 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min == 0 {
                    x_min = 1; // ignore non-positive values
                }

                (x_min.ilog2(), x_max.ilog2())
            },
        };

        Ok(IInterval::new_unsigned(IntType::U32, min as u128, max as u128))
    }
    /// Log10, which panics for values <= 0.
    pub fn ilog10(x: &IInterval) -> ArithResult {
        if x.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let ty = x.ty;

        let (min, max) = match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (mut x_min, x_max) = x.as_signed();
                if x_max <= 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min <= 0 {
                    x_min = 1; // ignore non-positive values
                }

                (x_min.ilog10(), x_max.ilog10())
            },
            IntTypeInfo::Unsigned(_) => {
                let (mut x_min, x_max) = x.as_unsigned();
                if x_max == 0 {
                    return Ok(IInterval::empty(IntType::U32));
                }
                if x_min == 0 {
                    x_min = 1; // ignore non-positive values
                }

                (x_min.ilog10(), x_max.ilog10())
            },
        };

        Ok(IInterval::new_unsigned(IntType::U32, min as u128, max as u128))
    }

    /// Power which saturates on overflow.
    pub fn saturating_pow(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        if rhs.ty != IntType::U32 {
            return Err(ArithError::TypeError);
        }

        let ty = lhs.ty;
        check_non_empty!(lhs, rhs);

        let (r_min, r_max) = rhs.as_unsigned();
        let (r_min, r_max) = (r_min as u32, r_max as u32);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();
                let l_has_zero = l_min <= 0 && 0 <= l_max;

                let single_r = |r: u32| -> IInterval {
                    if r == 0 {
                        // x^0 == 1, so return [1, 1]
                        return IInterval::single_signed(ty, 1);
                    }

                    if r % 2 == 0 {
                        // exponent is even
                        let pow_min = l_min.saturating_pow(r).min(t_max);
                        let pow_max = l_max.saturating_pow(r).min(t_max);

                        let max = pow_min.max(pow_max);
                        let min = if l_has_zero { 0 } else { pow_min.min(pow_max) };

                        IInterval::new_signed(ty, min, max)
                    } else {
                        IInterval::new_signed(
                            ty,
                            l_min.saturating_pow(r).clamp(t_min, t_max),
                            l_max.saturating_pow(r).clamp(t_min, t_max),
                        )
                    }
                };

                if r_min == r_max {
                    return Ok(single_r(r_min));
                }

                let mut result = single_r(r_min).hull_unwrap(&single_r(r_max));

                if r_min + 1 < r_max && l_min < 0 {
                    result = result.hull_unwrap(&single_r(r_min + 1));
                    result = result.hull_unwrap(&single_r(r_max - 1));
                }

                Ok(result)
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();

                if r_max == 0 {
                    // x^0 == 1, so return [1, 1]
                    return Ok(IInterval::single_unsigned(ty, 1));
                }
                if l_max == 0 {
                    if r_min == 0 {
                        return Ok(IInterval::new_unsigned(ty, 0, 1));
                    } else {
                        return Ok(IInterval::single_unsigned(ty, 0));
                    }
                }

                let min = if r_min == 0 && l_min == 0 {
                    0
                } else {
                    l_min.saturating_pow(r_min).min(t_max)
                };
                let max = l_max.saturating_pow(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Power which panics on overflow.
    pub fn strict_pow(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        if rhs.ty != IntType::U32 {
            return Err(ArithError::TypeError);
        }

        let ty = lhs.ty;
        check_non_empty!(lhs, rhs);

        let (r_min, r_max) = rhs.as_unsigned();
        let (mut r_min, r_max) = (r_min as u32, r_max as u32);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();

                let single_r = |r: u32| -> IInterval {
                    if r == 0 {
                        // x^0 == 1, so return [1, 1]
                        return IInterval::single_signed(ty, 1);
                    }

                    if r % 2 == 0 {
                        // exponent is even
                        let (min_pow, mut min_overflow) = l_min.overflowing_pow(r);
                        let (max_pow, mut max_overflow) = l_max.overflowing_pow(r);
                        min_overflow |= min_pow > t_max;
                        max_overflow |= max_pow > t_max;

                        let has_zero = l_min <= 0 && 0 <= l_max;
                        if has_zero {
                            return if min_overflow || max_overflow {
                                IInterval::new_signed(ty, 0, t_max)
                            } else {
                                IInterval::new_signed(ty, 0, min_pow.max(max_pow))
                            };
                        }

                        if min_overflow && max_overflow {
                            IInterval::empty(ty)
                        } else if min_overflow {
                            IInterval::new_signed(ty, max_pow, t_max)
                        } else if max_overflow {
                            IInterval::new_signed(ty, min_pow, t_max)
                        } else {
                            IInterval::new_signed(ty, min_pow.min(max_pow), max_pow.max(min_pow))
                        }
                    } else {
                        // exponent is odd
                        let (min_pow, min_overflow) = l_min.overflowing_pow(r);
                        let (max_pow, max_overflow) = l_max.overflowing_pow(r);

                        if l_min >= 0 {
                            return if min_overflow || min_pow > t_max {
                                IInterval::empty(ty)
                            } else if max_overflow || max_pow > t_max {
                                IInterval::new_signed(ty, min_pow, t_max)
                            } else {
                                IInterval::new_signed(ty, min_pow, max_pow)
                            };
                        }

                        if l_max <= 0 {
                            return if max_overflow || max_pow < t_min {
                                IInterval::empty(ty)
                            } else if min_overflow || min_pow < t_min {
                                IInterval::new_signed(ty, t_min, max_pow)
                            } else {
                                IInterval::new_signed(ty, min_pow, max_pow)
                            };
                        }

                        if min_overflow || min_pow < t_min {
                            return if max_overflow || max_pow > t_max {
                                IInterval::full(ty)
                            } else {
                                IInterval::new_signed(ty, t_min, max_pow)
                            };
                        }

                        if max_overflow || max_pow > t_max {
                            return IInterval::new_signed(ty, min_pow, t_max);
                        }

                        IInterval::new_signed(ty, min_pow, max_pow)
                    }
                };

                let min_range = single_r(r_min);
                if min_range.is_empty() || min_range.min == t_min && min_range.max == t_max {
                    // if the min range is empty, then the result is empty
                    // similarly, if the min range is the full range,
                    // then the result is the full range
                    return Ok(min_range);
                }

                if r_min == r_max {
                    return Ok(min_range);
                }

                let mut result = min_range.hull_unwrap(&single_r(r_min + 1));
                drop(min_range);

                if result.max == t_max && (result.min == t_min || l_min >= 0) {
                    // the result won't change anymore
                    return Ok(result);
                }

                if r_min + 1 == r_max {
                    return Ok(result);
                }

                // find actual max
                if result.max != t_max && l_max >= 0 {
                    if let Some(max_pow) = l_max.checked_pow(r_max).filter(|i| i <= &t_max) {
                        result.max = result.max.max(max_pow);
                    } else {
                        result.max = t_max;
                    }
                }
                if result.max != t_max && l_min < 0 {
                    // select the even integer in [r_max - 1, r_max]
                    let r_even = r_max & !1;
                    if let Some(min_pow) = l_min.checked_pow(r_even).filter(|i| i <= &t_max) {
                        result.max = result.max.max(min_pow);
                    } else {
                        result.max = t_max;
                    }
                }

                // find actual min
                if result.min != t_min && l_min < 0 {
                    // select the odd integer in [r_max - 1, r_max]
                    let r_odd = (r_max - 1) | 1;
                    if let Some(min_pow) = l_min.checked_pow(r_odd).filter(|i| i >= &t_min) {
                        result.min = result.min.min(min_pow);
                    } else {
                        result.min = t_min;
                    }
                }

                Ok(result)
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();

                if r_max == 0 {
                    // x^0 == 1, so return [1, 1]
                    return Ok(IInterval::single_unsigned(ty, 1));
                }

                let r_zero = if r_min == 0 {
                    r_min = 1;
                    // x^0 == 1, so return [1, 1]
                    IInterval::single_unsigned(ty, 1)
                } else {
                    IInterval::empty(ty)
                };

                let (min, min_overflow) = l_min.overflowing_pow(r_min);
                if min_overflow || min > t_max {
                    // it will always overflow
                    return Ok(IInterval::empty(ty));
                }
                let max = l_max.saturating_pow(r_max).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max).hull_unwrap(&r_zero))
            },
        }
    }
    /// Pow which wraps on overflow.
    pub fn wrapping_pow(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        if rhs.ty != IntType::U32 {
            return Err(ArithError::TypeError);
        }

        let ty = lhs.ty;
        check_non_empty!(lhs, rhs);

        let (r_min, r_max) = rhs.as_unsigned();
        let (_, r_max) = (r_min as u32, r_max as u32);

        match ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                let (l_min, l_max) = lhs.as_signed();

                let overflow = l_max.checked_pow(r_max).is_none_or(|i| i < t_min || i > t_max)
                    || l_min.checked_pow(r_max).is_none_or(|i| i < t_min || i > t_max);

                if overflow {
                    // it will overflow, so idk what to return
                    return Ok(IInterval::full(ty));
                }

                Self::strict_pow(lhs, rhs)
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (_, l_max) = lhs.as_unsigned();

                let (pow, pow_overflow) = l_max.overflowing_pow(r_max);
                if pow_overflow || pow > t_max {
                    // it's hard to know the true range when overflow happens
                    return Ok(IInterval::full(ty));
                }

                Self::strict_pow(lhs, rhs)
            },
        }
    }

    /// Absolute value which saturates on overflow.
    pub fn saturating_abs(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (x_min, x_max) = x.as_signed();

                if x_max <= 0 {
                    // already negative, so return the positive range
                    Ok(IInterval::new_signed(
                        x.ty,
                        x_max.saturating_neg().min(t_max),
                        x_min.saturating_neg().min(t_max),
                    ))
                } else if x_min >= 0 {
                    // already positive
                    Ok(x.clone())
                } else {
                    // contains zero, so return the positive range
                    Ok(IInterval::new_signed(
                        x.ty,
                        0,
                        x_max.max(x_min.saturating_neg()).min(t_max),
                    ))
                }
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }
    /// Absolute value which panics on overflow.
    pub fn strict_abs(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                let (mut x_min, x_max) = x.as_signed();

                if x_min >= 0 {
                    // already positive
                    Ok(x.clone())
                } else if x_max == t_min {
                    // all values in the range will overflow
                    Ok(IInterval::empty(x.ty))
                } else {
                    if x_min == t_min {
                        x_min += 1; // ignore value that will overflow
                    }

                    if x_max <= 0 {
                        // already negative, so return the positive range
                        Ok(IInterval::new_signed(x.ty, -x_max, -x_min))
                    } else {
                        Ok(IInterval::new_signed(x.ty, 0, x_max.max(-x_min)))
                    }
                }
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }
    /// Absolute value which wraps on overflow.
    pub fn wrapping_abs(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                // This is the same strict_abs, but with different handling of
                // the case where x_min == t_min.

                let strict = Self::strict_abs(x)?;

                let (x_min, _) = x.as_signed();

                if x_min == t_min {
                    let min_range = IInterval::single_signed(x.ty, t_min);

                    Ok(strict.hull_unwrap(&min_range))
                } else {
                    Ok(strict)
                }
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }
    /// Absolute value which never overflows, because it returns an unsigned value.
    pub fn unsigned_abs(x: &IInterval) -> ArithResult {
        match x.ty.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                debug_assert_eq!(t_min, -t_max - 1);

                if x.is_empty() {
                    return Ok(IInterval::empty(x.ty.swap_signedness()));
                }

                // This is the same strict_abs, but with different handling of
                // the case where x_min == t_min.

                let strict = Self::strict_abs(x)?.cast_signed_to_unsigned();

                let (x_min, _) = x.as_signed();

                if x_min == t_min {
                    let would_overflow = IInterval::single_unsigned(x.ty.swap_signedness(), t_max as u128 + 1);

                    Ok(strict.hull_unwrap(&would_overflow))
                } else {
                    Ok(strict)
                }
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }

    /// Absolute difference.
    pub fn abs_diff(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        let ret_ty = ty.to_unsigned();

        if lhs.is_empty() || rhs.is_empty() {
            return Ok(IInterval::empty(ret_ty));
        }

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let (min, max) = if l_max < r_min {
                    (r_min.abs_diff(l_max), r_max.abs_diff(l_min))
                } else if r_max < l_min {
                    (l_min.abs_diff(r_max), l_max.abs_diff(r_min))
                } else {
                    (0, u128::max(r_min.abs_diff(l_max), l_min.abs_diff(r_max)))
                };

                Ok(IInterval::new_unsigned(ret_ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let (min, max) = if l_max < r_min {
                    (r_min - l_max, r_max - l_min)
                } else if r_max < l_min {
                    (l_min - r_max, l_max - r_min)
                } else {
                    (0, u128::max(r_min.abs_diff(l_max), l_min.abs_diff(r_max)))
                };

                Ok(IInterval::new_unsigned(ret_ty, min, max))
            },
        }
    }

    /// Minimum.
    pub fn min(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = l_min.min(r_min);
                let max = l_max.min(r_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.min(r_min);
                let max = l_max.min(r_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }
    /// Maximum.
    pub fn max(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (l_min, l_max) = lhs.as_signed();
                let (r_min, r_max) = rhs.as_signed();

                let min = l_min.max(r_min);
                let max = l_max.max(r_max);

                Ok(IInterval::new_signed(ty, min, max))
            },
            IntTypeInfo::Unsigned(_) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (r_min, r_max) = rhs.as_unsigned();

                let min = l_min.max(r_min);
                let max = l_max.max(r_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Bitwise AND.
    pub fn and(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        fn and(lhs: &IInterval, rhs: &IInterval) -> IInterval {
            debug_assert_eq!(lhs.ty, rhs.ty);
            debug_assert!(!lhs.is_empty() && !rhs.is_empty());

            let l_bits = Bits::from_non_empty(lhs);
            let r_bits = Bits::from_non_empty(rhs);

            let zero = l_bits.zero & r_bits.zero;
            let one = l_bits.one & r_bits.one;

            Bits::new(zero, one).to_interval(lhs.ty)
        }

        if ty.is_signed() {
            let (l_min, l_max) = lhs.as_signed();
            let (r_min, r_max) = rhs.as_signed();
            let l_neg = l_max < 0;
            let r_neg = r_max < 0;
            if l_min < 0 && r_min < 0 {
                // Okay, so the problem here is that the `and` implementation is
                // only correct if both lhs and rhs have equal sign bits. So the
                // idea here is to split the ranges into negative and
                // non-negative parts, compute the `and` for each part separately,
                // and then combine the results.
                if !l_neg {
                    let l_n = IInterval::new_signed(ty, l_min, -1);
                    let l_p = IInterval::new_signed(ty, 0, l_max);

                    let result = if r_neg {
                        and(&l_n, rhs).hull_unwrap(&and(&l_p, rhs))
                    } else {
                        let r_n = IInterval::new_signed(ty, r_min, -1);
                        let r_p = IInterval::new_signed(ty, 0, r_max);
                        and(&l_n, &r_n)
                            .hull_unwrap(&and(&l_n, &r_p))
                            .hull_unwrap(&and(&l_p, &r_n))
                            .hull_unwrap(&and(&l_p, &r_p))
                    };
                    return Ok(result);
                }

                if !r_neg {
                    let r_n = IInterval::new_signed(ty, r_min, -1);
                    let r_p = IInterval::new_signed(ty, 0, r_max);

                    return Ok(and(lhs, &r_n).hull_unwrap(&and(lhs, &r_p)));
                }
            }
            Ok(and(lhs, rhs))
        } else {
            Ok(and(lhs, rhs))
        }
    }
    /// Bitwise OR.
    pub fn or(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        if ty.is_signed() {
            Self::not(&Self::and(&Self::not(lhs)?, &Self::not(rhs)?)?)
        } else {
            let l_bits = Bits::from_non_empty(lhs);
            let r_bits = Bits::from_non_empty(rhs);

            let zero = l_bits.zero | r_bits.zero;
            let one = l_bits.one | r_bits.one;

            let (mut min, mut max) = (zero, one);
            debug_assert_eq!(
                IInterval::new_unsigned(ty, min, max),
                Bits::new(zero, one).to_interval(ty)
            );

            // This narrows the range using:
            //   max(a,b) <= a|b <= a + b
            let (l_min, l_max) = lhs.as_unsigned();
            let (r_min, r_max) = rhs.as_unsigned();
            max = max.min(l_max.saturating_add(r_max));
            min = min.max(l_min).max(r_min);

            Ok(IInterval::new_unsigned(ty, min, max))
        }
    }
    /// Bitwise XOR.
    pub fn xor(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        let l_bits = Bits::from_non_empty(lhs);
        let r_bits = Bits::from_non_empty(rhs);

        // bits that are different in lhs and rhs
        let l_diff = l_bits.zero ^ l_bits.one;
        let r_diff = r_bits.zero ^ r_bits.one;
        let diff = l_diff | r_diff;

        let xor = l_bits.zero ^ r_bits.zero;
        let zero = xor & !diff;
        let one = xor | diff;

        Ok(Bits::new(zero, one).to_interval(ty))
    }
    /// Bitwise NOT.
    pub fn not(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        let ty = x.ty;

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (x_min, x_max) = x.as_signed();

                // maybe the only operation where signed is simpler than unsigned
                Ok(IInterval::new_signed(ty, !x_max, !x_min))
            },
            IntTypeInfo::Unsigned(t_max) => {
                let (x_min, x_max) = x.as_unsigned();

                Ok(IInterval::new_unsigned(ty, !x_max & t_max, !x_min & t_max))
            },
        }
    }

    pub fn strict_shl(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        check_non_empty!(lhs, rhs);

        let ty = lhs.ty;
        let bit_width = ty.bits();

        let Some((r_min, r_max)) = parse_shift_strict(rhs, bit_width) else {
            return Ok(IInterval::empty(ty));
        };

        let mask = !u128::MAX.unbounded_shl(bit_width as u32);

        let mut bits = Bits::from_non_empty(lhs);
        bits.zero = (bits.zero << r_min) & mask;
        bits.one = (bits.one << r_min) & mask;

        let mut result = bits.to_interval(ty);
        for _ in r_min..r_max {
            bits.zero = (bits.zero << 1) & mask;
            bits.one = (bits.one << 1) & mask;
            result = result.hull_unwrap(&bits.to_interval(ty));
        }

        Ok(result)
    }
    pub fn wrapping_shl(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        check_non_empty!(lhs, rhs);

        let ty = lhs.ty;
        let bit_width = ty.bits();

        let Some((r_min, r_max)) = parse_shift_wrapping(rhs, bit_width) else {
            return Ok(IInterval::empty(ty));
        };

        let result = if r_min <= r_max {
            Self::strict_shl(
                lhs,
                &IInterval::new_unsigned(IntType::U32, r_min as u128, r_max as u128),
            )?
        } else {
            let left = Self::strict_shl(lhs, &IInterval::new_unsigned(IntType::U32, 0, r_max as u128))?;
            let right = Self::strict_shl(
                lhs,
                &IInterval::new_unsigned(IntType::U32, r_min as u128, (bit_width - 1) as u128),
            )?;

            left.hull_unwrap(&right)
        };

        Ok(result)
    }
    pub fn unbounded_shl(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        if rhs.ty != IntType::U32 {
            return Err(ArithError::TypeError);
        }

        check_non_empty!(lhs, rhs);

        let mut result = Self::strict_shl(lhs, rhs)?;

        let ty = lhs.ty;

        let (_, r_max) = rhs.as_unsigned();
        if r_max as u32 >= ty.bits() as u32 {
            let zero = if ty.is_signed() {
                IInterval::single_signed(ty, 0)
            } else {
                IInterval::single_unsigned(ty, 0)
            };
            result = result.hull_unwrap(&zero);
        }

        Ok(result)
    }

    pub fn strict_shr(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        check_non_empty!(lhs, rhs);

        let ty = lhs.ty;
        let bit_width = ty.bits();

        let Some((r_min, r_max)) = parse_shift_strict(rhs, bit_width) else {
            return Ok(IInterval::empty(ty));
        };

        if ty.is_signed() {
            Ok(split_by_sign_bit_signed(lhs, |min, max, sign| {
                if sign == SignBit::NonNeg {
                    IInterval::new_signed(ty, min >> r_max, max >> r_min)
                } else {
                    IInterval::new_signed(ty, min >> r_min, max >> r_max)
                }
            }))
        } else {
            let (l_min, l_max) = lhs.as_unsigned();

            Ok(IInterval::new_unsigned(ty, l_min >> r_max, l_max >> r_min))
        }
    }
    pub fn wrapping_shr(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        check_non_empty!(lhs, rhs);

        let ty = lhs.ty;
        let bit_width = ty.bits();

        let Some((r_min, r_max)) = parse_shift_wrapping(rhs, bit_width) else {
            return Ok(IInterval::empty(ty));
        };

        if r_min <= r_max {
            Self::strict_shr(
                lhs,
                &IInterval::new_unsigned(IntType::U32, r_min as u128, r_max as u128),
            )
        } else {
            Self::strict_shr(lhs, &IInterval::new_unsigned(IntType::U32, 0, (bit_width - 1) as u128))
        }
    }
    pub fn unbounded_shr(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        if rhs.ty != IntType::U32 {
            return Err(ArithError::TypeError);
        }

        check_non_empty!(lhs, rhs);

        let mut result = Self::strict_shr(lhs, rhs)?;

        let ty = lhs.ty;

        let (_, r_max) = rhs.as_unsigned();
        if r_max as u32 >= ty.bits() as u32 {
            let zero = if ty.is_signed() {
                let has_neg = lhs.min < 0;
                let has_pos = lhs.max >= 0;
                if has_neg {
                    if has_pos {
                        IInterval::new_signed(ty, -1, 0)
                    } else {
                        IInterval::single_signed(ty, -1)
                    }
                } else {
                    IInterval::single_signed(ty, 0)
                }
            } else {
                IInterval::single_unsigned(ty, 0)
            };
            result = result.hull_unwrap(&zero);
        }

        Ok(result)
    }

    pub fn leading_zeros(x: &IInterval) -> ArithResult {
        if x.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let bit_width = x.ty.bits() as u32;
        let padding = 128 - bit_width;

        Ok(split_by_sign_bit(x, |min, max| {
            let r_min = max.leading_zeros().saturating_sub(padding);
            let r_max = min.leading_zeros().saturating_sub(padding);

            IInterval::new_unsigned(IntType::U32, r_min as u128, r_max as u128)
        }))
    }
    pub fn leading_ones(x: &IInterval) -> ArithResult {
        Self::leading_zeros(&Self::not(x)?)
    }
    pub fn trailing_zeros(x: &IInterval) -> ArithResult {
        if x.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let bit_width = x.ty.bits() as u32;

        Ok(split_by_sign_bit(x, |min, max| {
            if min == max {
                let trailing = min.trailing_zeros().min(bit_width);
                return IInterval::single_unsigned(IntType::U32, trailing as u128);
            }

            // if min != max, then the range contains at least one odd value,
            // so the minimum trailing zeros is 0

            if min == 0 {
                // 0 is all 0s
                return IInterval::new_unsigned(IntType::U32, 0, bit_width as u128);
            }

            let mut a = min;
            let mut b = max & !1; // make sure max isn't u128::MAX

            let mut scale: u32 = 0;
            while a != b {
                scale += 1;
                a = (a + 1) >> 1;
                b >>= 1;
            }

            let most_even = a << scale;

            let r_max = most_even.trailing_zeros();

            IInterval::new_unsigned(IntType::U32, 0, r_max as u128)
        }))
    }
    pub fn trailing_ones(x: &IInterval) -> ArithResult {
        Self::trailing_zeros(&Self::not(x)?)
    }
    pub fn count_ones(x: &IInterval) -> ArithResult {
        if x.is_empty() {
            return Ok(IInterval::empty(IntType::U32));
        }

        let bit_width = x.ty.bits() as u32;

        Ok(split_by_sign_bit(x, |min, max| {
            let equal_prefix = (min ^ max).leading_zeros();
            let mut spread = 128 - equal_prefix;

            let mask = u128::MAX.unbounded_shl(bit_width);
            let fixed_ones = (min & !mask).unbounded_shr(spread).count_ones();

            let r_min = if min == 0 { 0 } else { 1 };

            if max | u128::MAX.unbounded_shl(spread) != u128::MAX {
                spread -= 1;
            }

            IInterval::new_unsigned(
                IntType::U32,
                fixed_ones.min(bit_width).max(r_min) as u128,
                (fixed_ones + spread).min(bit_width) as u128,
            )
        }))
    }
    pub fn count_zeros(x: &IInterval) -> ArithResult {
        Self::count_ones(&Self::not(x)?)
    }

    pub fn signum(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        let ty = x.ty;

        match ty.info() {
            IntTypeInfo::Signed(_, _) => {
                let (min, max) = x.as_signed();

                if min > 0 {
                    Ok(IInterval::single_signed(ty, 1))
                } else if max < 0 {
                    Ok(IInterval::single_signed(ty, -1))
                } else {
                    let min = if min < 0 { -1 } else { 0 };
                    let max = if max > 0 { 1 } else { 0 };
                    Ok(IInterval::new_signed(ty, min, max))
                }
            },
            IntTypeInfo::Unsigned(_) => Err(ArithError::Unsupported),
        }
    }

    /// Next power of two which panics on overflow.
    pub fn strict_next_power_of_two(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        let ty = x.ty;

        match ty.info() {
            IntTypeInfo::Signed(_, _) => Err(ArithError::Unsupported),
            IntTypeInfo::Unsigned(t_max) => {
                let (x_min, x_max) = x.as_unsigned();

                let min = x_min.checked_next_power_of_two().filter(|i| i <= &t_max);
                let max = x_max.checked_next_power_of_two().filter(|i| i <= &t_max);

                let result = match (min, max) {
                    (Some(min), Some(max)) => IInterval::new_unsigned(ty, min, max),
                    (Some(min), None) => IInterval::new_unsigned(ty, min, t_max ^ (t_max >> 1)),
                    (None, _) => IInterval::empty(ty),
                };

                Ok(result)
            },
        }
    }
    /// Next power of two which wraps on overflow.
    pub fn wrapping_next_power_of_two(x: &IInterval) -> ArithResult {
        check_non_empty!(x);

        let ty = x.ty;

        match ty.info() {
            IntTypeInfo::Signed(_, _) => Err(ArithError::Unsupported),
            IntTypeInfo::Unsigned(t_max) => {
                let (x_min, x_max) = x.as_unsigned();

                let min = x_min.checked_next_power_of_two().filter(|i| i <= &t_max);
                let max = x_max.checked_next_power_of_two().filter(|i| i <= &t_max);

                let result = match (min, max) {
                    (Some(min), Some(max)) => IInterval::new_unsigned(ty, min, max),
                    (Some(_), None) => IInterval::new_unsigned(ty, 0, t_max ^ (t_max >> 1)),
                    (None, _) => IInterval::single_unsigned(ty, 0),
                };

                Ok(result)
            },
        }
    }

    /// Next multiple of which panics on overflow.
    pub fn strict_next_multiple_of(lhs: &IInterval, rhs: &IInterval) -> ArithResult {
        let ty = check_same_ty(lhs, rhs)?;
        check_non_empty!(lhs, rhs);

        match ty.info() {
            IntTypeInfo::Signed(_, _) => Err(ArithError::Unsupported),
            IntTypeInfo::Unsigned(t_max) => {
                let (l_min, l_max) = lhs.as_unsigned();
                let (mut r_min, r_max) = rhs.as_unsigned();

                if r_max == 0 {
                    // x % 0 panics
                    return Ok(IInterval::empty(ty));
                }
                if r_min == 0 {
                    r_min = 1;
                }

                if r_min == r_max {
                    let r = r_min;

                    // This is a lot easier if rhs is a constant.
                    let Some(min) = l_min.checked_next_multiple_of(r).filter(|i| i <= &t_max) else {
                        return Ok(IInterval::empty(ty));
                    };
                    let max = l_max
                        .checked_next_multiple_of(r)
                        .map(|i| if i > t_max { i - r } else { i })
                        .unwrap_or(t_max);

                    return Ok(IInterval::new_unsigned(ty, min, max));
                }

                let min = l_min;
                let max = l_max.saturating_add(r_max - 1).min(t_max);

                Ok(IInterval::new_unsigned(ty, min, max))
            },
        }
    }

    /// Casts unsigned to signed.
    pub fn cast_signed(x: &IInterval) -> ArithResult {
        if x.ty.is_signed() {
            return Err(ArithError::Unsupported);
        }

        Ok(x.cast_unsigned_to_signed())
    }
    /// Casts signed to unsigned.
    pub fn cast_unsigned(x: &IInterval) -> ArithResult {
        if !x.ty.is_signed() {
            return Err(ArithError::Unsupported);
        }

        Ok(x.cast_signed_to_unsigned())
    }

    pub fn cast_as(x: &IInterval, target: IntType) -> ArithResult {
        if x.ty == target {
            return Ok(x.clone());
        }
        if x.is_empty() {
            return Ok(IInterval::empty(target));
        }

        let src_width = x.ty.bits();
        let target_width = target.bits();
        let src_signed = x.ty.is_signed();
        let target_signed = target.is_signed();

        let target_same_sign = if src_signed != target_signed {
            target.swap_signedness()
        } else {
            target
        };

        let src: IInterval = if src_width < target_width {
            // widening cast
            x.cast_widen(target_same_sign)
        } else if src_width > target_width {
            // narrowing cast
            match target_same_sign.info() {
                IntTypeInfo::Signed(t_min, t_max) => {
                    let mask = (t_max.cast_unsigned() << 1) | 1;
                    split_by_sign_bit(x, |min, max| {
                        if max - min >= mask {
                            IInterval::new_signed(target_same_sign, t_min, t_max)
                        } else {
                            let min = min & mask;
                            let max = max & mask;
                            if min > max {
                                IInterval::new_signed(target_same_sign, t_min, t_max)
                            } else {
                                let unsigned = target_same_sign.swap_signedness();
                                IInterval::new_unsigned(unsigned, min, max).cast_unsigned_to_signed()
                            }
                        }
                    })
                },
                IntTypeInfo::Unsigned(t_max) => {
                    let (s_min, s_max) = x.as_unsigned();

                    if s_max - s_min >= t_max {
                        IInterval::new_unsigned(target_same_sign, 0, t_max)
                    } else {
                        let min = s_min & t_max;
                        let max = s_max & t_max;
                        if min > max {
                            IInterval::new_unsigned(target_same_sign, 0, t_max)
                        } else {
                            IInterval::new_unsigned(target_same_sign, min, max)
                        }
                    }
                },
            }
        } else {
            // only signedness cast
            x.clone()
        };

        // cast to target signedness
        let result = if src_signed != target_signed {
            if target_signed {
                src.cast_unsigned_to_signed()
            } else {
                src.cast_signed_to_unsigned()
            }
        } else {
            src
        };

        Ok(result)
    }
}
