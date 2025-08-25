use clippy_utils::rinterval::IInterval;
use clippy_utils::ty::{EnumValue, read_explicit_enum_value};
use rustc_middle::ty::{self, AdtDef, IntTy, Ty, TyCtxt, UintTy, VariantDiscr};

/// Returns the size in bits of an integral type, or `None` if `ty` is not an
/// integral type.
pub(super) fn int_ty_to_nbits(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<u64> {
    match ty.kind() {
        ty::Int(IntTy::Isize) | ty::Uint(UintTy::Usize) => Some(tcx.data_layout.pointer_size().bits()),
        ty::Int(i) => i.bit_width(),
        ty::Uint(i) => i.bit_width(),
        _ => None,
    }
}

pub(super) fn enum_value_nbits(value: EnumValue) -> u64 {
    match value {
        EnumValue::Unsigned(x) => 128 - x.leading_zeros(),
        EnumValue::Signed(x) if x < 0 => 128 - (-(x + 1)).leading_zeros() + 1,
        EnumValue::Signed(x) => 128 - x.leading_zeros(),
    }
    .into()
}

pub(super) fn enum_ty_to_nbits(adt: AdtDef<'_>, tcx: TyCtxt<'_>) -> u64 {
    let mut explicit = 0i128;
    let (start, end) = adt
        .variants()
        .iter()
        .fold((0, i128::MIN), |(start, end), variant| match variant.discr {
            VariantDiscr::Relative(x) => match explicit.checked_add(i128::from(x)) {
                Some(x) => (start, end.max(x)),
                None => (i128::MIN, end),
            },
            VariantDiscr::Explicit(id) => match read_explicit_enum_value(tcx, id) {
                Some(EnumValue::Signed(x)) => {
                    explicit = x;
                    (start.min(x), end.max(x))
                },
                Some(EnumValue::Unsigned(x)) => match i128::try_from(x) {
                    Ok(x) => {
                        explicit = x;
                        (start, end.max(x))
                    },
                    Err(_) => (i128::MIN, end),
                },
                None => (start, end),
            },
        });

    if start > end {
        // No variants.
        0
    } else {
        let neg_bits = if start < 0 {
            128 - (-(start + 1)).leading_zeros() + 1
        } else {
            0
        };
        let pos_bits = if end > 0 { 128 - end.leading_zeros() } else { 0 };
        neg_bits.max(pos_bits).into()
    }
}

pub(super) fn format_cast_operand(range: IInterval) -> String {
    if range.is_empty() {
        // This is the weird edge cast where we know that the cast will never
        // actually happen, because it's unreachable.
        return "the cast operand is unreachable".to_string();
    }

    if range.ty.is_signed() {
        let (min, max) = range.as_signed();

        if min == max {
            format!("the cast operand is `{min}`")
        } else {
            let min = PrettyNumber::from(min);
            let max = PrettyNumber::from(max);
            format!("the cast operand may contain values in the range `{min}..={max}`")
        }
    } else {
        let (min, max) = range.as_unsigned();

        if min == max {
            format!("the cast operand is `{min}`")
        } else {
            let min = PrettyNumber::from(min);
            let max = PrettyNumber::from(max);
            format!("the cast operand may contain values in the range `{min}..={max}`")
        }
    }
}
enum PrettyNumber {
    Unsigned(u128),
    Signed(i128),
}
impl PrettyNumber {
    fn abs(&self) -> u128 {
        match self {
            PrettyNumber::Unsigned(value) => *value,
            PrettyNumber::Signed(value) => value.unsigned_abs(),
        }
    }
    fn is_negative(&self) -> bool {
        match self {
            PrettyNumber::Unsigned(_) => false,
            PrettyNumber::Signed(value) => value.is_negative(),
        }
    }
}
impl From<u128> for PrettyNumber {
    fn from(value: u128) -> Self {
        PrettyNumber::Unsigned(value)
    }
}
impl From<i128> for PrettyNumber {
    fn from(value: i128) -> Self {
        PrettyNumber::Signed(value)
    }
}
impl std::fmt::Display for PrettyNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let abs = self.abs();
        if abs > 4096 + 100 {
            // This is the closest power of 2 minus 1.
            // The minus 1 is necessary, because we can't represent 2^128.
            let mut closest_power_of_two_m1 = abs.checked_next_power_of_two().unwrap_or(0).wrapping_sub(1);
            if closest_power_of_two_m1.abs_diff(abs) > 100 {
                closest_power_of_two_m1 /= 2;
            }
            if closest_power_of_two_m1.abs_diff(abs) < 100 {
                let mut diff = abs.wrapping_sub(closest_power_of_two_m1.wrapping_add(1)).cast_signed() as i32;
                if self.is_negative() {
                    write!(f, "-")?;
                    diff = -diff;
                }

                let power = closest_power_of_two_m1.count_ones();
                write!(f, "2^{power}")?;

                if diff < 0 {
                    write!(f, "{diff}")?;
                } else if diff > 0 {
                    write!(f, "+{diff}")?;
                }
                return Ok(());
            }
        }

        match self {
            PrettyNumber::Unsigned(value) => write!(f, "{value}"),
            PrettyNumber::Signed(value) => write!(f, "{value}"),
        }
    }
}
