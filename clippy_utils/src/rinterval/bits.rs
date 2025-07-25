use super::{IInterval, IntType};

/// A representation of the equal bits of an integer interval.
///
/// This struct has 2 main fields: `zero` and `one`. They both represent the
/// equal bits, but they handle unequal bits differently. Unequal bits are
/// represented as `0` in `zero` and `1` in `one`.
///
/// So e.g. if there is only one value in the interval, then all bits are
/// equal and `zero` and `one` will be equal. Similarly, if the interval
/// contains all values of the type, then `zero` will be all 0s and `one`
/// will be all 1s since all bits are different.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub(crate) struct Bits {
    pub zero: u128,
    pub one: u128,
}
impl Bits {
    pub const fn new(zero: u128, one: u128) -> Self {
        debug_assert!(one & zero == zero);
        debug_assert!(one | zero == one);

        Self { zero, one }
    }
    pub const fn from_non_empty(i: &IInterval) -> Self {
        debug_assert!(!i.is_empty());

        let min = i.min.cast_unsigned();
        let max = i.max.cast_unsigned();

        // all bits that are the same will be 0 in this mask
        let equal_bits = min ^ max;

        // number of buts that are the same
        let equal = equal_bits.leading_zeros();

        // mask for all unequal bits
        let unequal_mask = u128::MAX.unbounded_shr(equal);

        let zero = min & !unequal_mask;
        let one = max | unequal_mask;

        Self::new(zero, one)
    }

    pub const fn to_interval(&self, ty: IntType) -> IInterval {
        if ty.is_signed() {
            let u_ty = ty.swap_signedness();
            let u_max = u_ty.max_value().cast_unsigned();
            IInterval::new_unsigned(u_ty, self.zero & u_max, self.one & u_max).cast_unsigned_to_signed()
        } else {
            #[cfg(debug_assertions)]
            {
                let u_max = ty.max_value().cast_unsigned();
                debug_assert!(self.zero <= u_max);
                debug_assert!(self.one <= u_max);
            }

            IInterval::new_unsigned(ty, self.zero, self.one)
        }
    }
}
impl std::fmt::Display for Bits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IntBits[")?;

        let mut zero = self.zero.reverse_bits();
        let mut one = self.one.reverse_bits();

        for chunk_32 in 0..4 {
            if chunk_32 > 0 {
                write!(f, " ")?;
            }

            let z_32 = zero as u32;
            let o_32 = one as u32;
            if z_32 == o_32 {
                if z_32 == 0 {
                    write!(f, "0_x32")?;
                    continue;
                } else if z_32 == u32::MAX {
                    write!(f, "1_x32")?;
                    continue;
                }
            }
            if z_32 == !o_32 {
                write!(f, "?_x32")?;
                continue;
            }

            for chunk_4 in 0..8 {
                if chunk_4 > 0 {
                    write!(f, "_")?;
                }

                for _ in 0..4 {
                    let z = zero & 1;
                    let o = one & 1;

                    if z == o {
                        write!(f, "{}", z as u8)?;
                    } else {
                        write!(f, "?")?;
                    }

                    zero >>= 1;
                    one >>= 1;
                }
            }
        }

        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_bits_for_single_values() {
        fn test(i: IInterval) {
            let bits = Bits::from_non_empty(&i);
            let back = bits.to_interval(i.ty);
            assert_eq!(i, back);
        }

        for x in i8::MIN..=i8::MAX {
            test(IInterval::single_signed(IntType::I8, x as i128));
        }
        for x in u8::MIN..=u8::MAX {
            test(IInterval::single_unsigned(IntType::U8, x as u128));
        }
    }

    #[test]
    fn test_superset_for_ranges() {
        fn test(i: IInterval) {
            let bits = Bits::from_non_empty(&i);
            let back = bits.to_interval(i.ty);
            assert!(
                back.is_superset_of(&i),
                "Expected {back} to be a superset of {i} for bits {bits}"
            );
        }

        for min in i8::MIN..i8::MAX {
            for max in min + 1..=i8::MAX {
                test(IInterval::new_signed(IntType::I8, min as i128, max as i128));
            }
        }
        for min in u8::MIN..u8::MAX {
            for max in min + 1..=u8::MAX {
                test(IInterval::new_unsigned(IntType::U8, min as u128, max as u128));
            }
        }
    }
}
