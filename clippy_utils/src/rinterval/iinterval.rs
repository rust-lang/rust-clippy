#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i16)]
#[must_use]
pub enum IntType {
    U8 = 8,
    U16 = 16,
    U32 = 32,
    U64 = 64,
    U128 = 128,
    I8 = -8,
    I16 = -16,
    I32 = -32,
    I64 = -64,
    I128 = -128,
}
impl IntType {
    pub const fn is_signed(self) -> bool {
        (self as i16) < 0
    }
    pub const fn bits(self) -> u8 {
        (self as i16).unsigned_abs() as u8
    }
    pub const fn min_value(self) -> i128 {
        match self {
            IntType::U8 => 0,
            IntType::U16 => 0,
            IntType::U32 => 0,
            IntType::U64 => 0,
            IntType::U128 => 0,
            IntType::I8 => i8::MIN as i128,
            IntType::I16 => i16::MIN as i128,
            IntType::I32 => i32::MIN as i128,
            IntType::I64 => i64::MIN as i128,
            IntType::I128 => i128::MIN,
        }
    }
    pub const fn max_value(self) -> i128 {
        match self {
            IntType::U8 => u8::MAX as i128,
            IntType::U16 => u16::MAX as i128,
            IntType::U32 => u32::MAX as i128,
            IntType::U64 => u64::MAX as i128,
            IntType::U128 => u128::MAX as i128,
            IntType::I8 => i8::MAX as i128,
            IntType::I16 => i16::MAX as i128,
            IntType::I32 => i32::MAX as i128,
            IntType::I64 => i64::MAX as i128,
            IntType::I128 => i128::MAX,
        }
    }
    pub(crate) const fn info(self) -> IntTypeInfo {
        match self {
            IntType::U8 => IntTypeInfo::Unsigned(u8::MAX as u128),
            IntType::U16 => IntTypeInfo::Unsigned(u16::MAX as u128),
            IntType::U32 => IntTypeInfo::Unsigned(u32::MAX as u128),
            IntType::U64 => IntTypeInfo::Unsigned(u64::MAX as u128),
            IntType::U128 => IntTypeInfo::Unsigned(u128::MAX),
            IntType::I8 => IntTypeInfo::Signed(i8::MIN as i128, i8::MAX as i128),
            IntType::I16 => IntTypeInfo::Signed(i16::MIN as i128, i16::MAX as i128),
            IntType::I32 => IntTypeInfo::Signed(i32::MIN as i128, i32::MAX as i128),
            IntType::I64 => IntTypeInfo::Signed(i64::MIN as i128, i64::MAX as i128),
            IntType::I128 => IntTypeInfo::Signed(i128::MIN, i128::MAX),
        }
    }

    pub const fn swap_signedness(self) -> IntType {
        match self {
            IntType::U8 => IntType::I8,
            IntType::U16 => IntType::I16,
            IntType::U32 => IntType::I32,
            IntType::U64 => IntType::I64,
            IntType::U128 => IntType::I128,
            IntType::I8 => IntType::U8,
            IntType::I16 => IntType::U16,
            IntType::I32 => IntType::U32,
            IntType::I64 => IntType::U64,
            IntType::I128 => IntType::U128,
        }
    }
    pub const fn to_signed(self) -> IntType {
        match self {
            IntType::U8 | IntType::I8 => IntType::I8,
            IntType::U16 | IntType::I16 => IntType::I16,
            IntType::U32 | IntType::I32 => IntType::I32,
            IntType::U64 | IntType::I64 => IntType::I64,
            IntType::U128 | IntType::I128 => IntType::I128,
        }
    }
    pub const fn to_unsigned(self) -> IntType {
        match self {
            IntType::U8 | IntType::I8 => IntType::U8,
            IntType::U16 | IntType::I16 => IntType::U16,
            IntType::U32 | IntType::I32 => IntType::U32,
            IntType::U64 | IntType::I64 => IntType::U64,
            IntType::U128 | IntType::I128 => IntType::U128,
        }
    }
}

pub(crate) enum IntTypeInfo {
    Signed(i128, i128),
    Unsigned(u128),
}

/// Represents a range of values for an integer type.
///
/// ## Exactness
///
/// Ranges must generally be assumed to be **inexact**. It is simply not
/// possible to accurately represent the set of all possible values of an
/// integer expression using only its minimum and maximum values.
///
/// As such, this type represents a **superset** of the actual set of values of
/// an expression.

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct IInterval {
    pub ty: IntType,
    pub min: i128,
    pub max: i128,
}

impl IInterval {
    pub const fn new_signed(ty: IntType, min: i128, max: i128) -> Self {
        #[cfg(debug_assertions)]
        {
            debug_assert!(min <= max);
            debug_assert!(ty.is_signed());
            if let IntTypeInfo::Signed(t_min, t_max) = ty.info() {
                debug_assert!(min >= t_min);
                debug_assert!(max <= t_max);
            }
        }

        Self { ty, min, max }
    }
    pub const fn new_unsigned(ty: IntType, min: u128, max: u128) -> Self {
        #[cfg(debug_assertions)]
        {
            debug_assert!(min <= max);
            debug_assert!(!ty.is_signed());
            if let IntTypeInfo::Unsigned(t_max) = ty.info() {
                debug_assert!(max <= t_max);
            }
        }

        Self {
            ty,
            min: min.cast_signed(),
            max: max.cast_signed(),
        }
    }
    pub const fn single_signed(ty: IntType, value: i128) -> Self {
        Self::new_signed(ty, value, value)
    }
    pub const fn single_unsigned(ty: IntType, value: u128) -> Self {
        Self::new_unsigned(ty, value, value)
    }

    /// Creates an empty interval for the given integer type.
    pub const fn empty(ty: IntType) -> Self {
        Self { ty, min: 1, max: 0 }
    }
    /// Creates the smallest interval that contains all possible values of the
    /// given integer type.
    pub const fn full(ty: IntType) -> Self {
        match ty.info() {
            IntTypeInfo::Signed(min, max) => Self::new_signed(ty, min, max),
            IntTypeInfo::Unsigned(max) => Self::new_unsigned(ty, 0, max),
        }
    }

    /// Whether the interval contains no values.
    pub const fn is_empty(&self) -> bool {
        if self.ty.is_signed() {
            let min = self.min;
            let max = self.max;
            min > max
        } else {
            let min = self.min.cast_unsigned();
            let max = self.max.cast_unsigned();
            min > max
        }
    }
    pub fn is_full(&self) -> bool {
        self == &Self::full(self.ty)
    }

    /// Returns whether the interval contains at least one negative value.
    pub fn contains_negative(&self) -> bool {
        if self.is_empty() || !self.ty.is_signed() {
            false
        } else {
            let (min, _) = self.as_signed();
            min < 0
        }
    }
    /// Returns whether all values in the interval can be represented by the
    /// given target type.
    pub fn fits_into(&self, target: IntType) -> bool {
        if self.is_empty() || self.ty == target {
            return true; // empty set fits into any type, and same type always fits
        }

        match target.info() {
            IntTypeInfo::Signed(t_min, t_max) => {
                if self.ty.is_signed() {
                    let (min, max) = self.as_signed();
                    t_min <= min && max <= t_max
                } else {
                    let (_, max) = self.as_unsigned();
                    max <= t_max.cast_unsigned()
                }
            },
            IntTypeInfo::Unsigned(t_max) => {
                if self.ty.is_signed() {
                    let (min, max) = self.as_signed();
                    min >= 0 && max.cast_unsigned() <= t_max
                } else {
                    let (_, max) = self.as_unsigned();
                    max <= t_max
                }
            },
        }
    }

    /// Returns the minimum and maximum values for intervals of signed types.
    ///
    /// If the interval is empty or the type is unsigned, the result is
    /// implementation-defined.
    pub const fn as_signed(&self) -> (i128, i128) {
        debug_assert!(self.ty.is_signed());
        debug_assert!(!self.is_empty());
        (self.min, self.max)
    }
    /// Returns the minimum and maximum values for intervals of unsigned types.
    ///
    /// If the interval is empty or the type is signed, the result is unspecified.
    pub const fn as_unsigned(&self) -> (u128, u128) {
        debug_assert!(!self.ty.is_signed());
        debug_assert!(!self.is_empty());
        (self.min.cast_unsigned(), self.max.cast_unsigned())
    }

    /// Returns the smallest interval that contains both `self` and `other`.
    ///
    /// The result is unspecified if the two intervals have different types.
    pub fn hull_unwrap(&self, other: &Self) -> Self {
        debug_assert!(self.ty == other.ty);

        if self.is_empty() {
            return other.clone();
        }
        if other.is_empty() {
            return self.clone();
        }

        if self.ty.is_signed() {
            let min = self.min.min(other.min);
            let max = self.max.max(other.max);
            Self::new_signed(self.ty, min, max)
        } else {
            let (l_min, l_max) = self.as_unsigned();
            let (r_min, r_max) = other.as_unsigned();

            let min = l_min.min(r_min);
            let max = l_max.max(r_max);
            Self::new_unsigned(self.ty, min, max)
        }
    }
    /// Returns the smallest interval that contains both `self` and `other`.
    ///
    /// Returns `None` if the two intervals have different types.
    pub fn hull(&self, other: &Self) -> Option<Self> {
        if self.ty != other.ty {
            return None;
        }
        Some(self.hull_unwrap(other))
    }

    /// Returns whether all values in `self` are also contained in `other`.
    ///
    /// The result is unspecified if the two intervals have types of different
    /// signedness.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        debug_assert!(self.ty.is_signed() == other.ty.is_signed());

        if self.is_empty() {
            return true; // Empty set is a subset of any set
        }
        if other.is_empty() {
            return false; // Non-empty set cannot be a subset of an empty set
        }

        if self.ty.is_signed() {
            self.min >= other.min && self.max <= other.max
        } else {
            let (l_min, l_max) = self.as_unsigned();
            let (r_min, r_max) = other.as_unsigned();
            l_min >= r_min && l_max <= r_max
        }
    }
    /// Same as `is_subset_of`, but checks if `self` is a superset of `other`.
    pub fn is_superset_of(&self, other: &Self) -> bool {
        other.is_subset_of(self)
    }

    pub fn to_string_untyped(&self) -> String {
        if self.is_empty() {
            "<empty>".to_string()
        } else if self.ty.is_signed() {
            let (min, max) = self.as_signed();
            if min == max {
                format!("{min}")
            } else {
                format!("{min}..={max}")
            }
        } else {
            let (min, max) = self.as_unsigned();
            if min == max {
                format!("{min}")
            } else {
                format!("{min}..={max}")
            }
        }
    }

    /// Casts an unsigned interval to a signed one of a type with the same bit width.
    ///
    /// If the type is already signed, the result is unspecified.
    pub(crate) const fn cast_unsigned_to_signed(&self) -> Self {
        debug_assert!(!self.ty.is_signed());

        let target = self.ty.swap_signedness();
        if self.is_empty() {
            return Self::empty(target);
        }

        let t_max = target.max_value().cast_unsigned();
        let (x_min, x_max) = self.as_unsigned();

        if x_min > t_max {
            IInterval::new_signed(target, (x_min | !t_max).cast_signed(), (x_max | !t_max).cast_signed())
        } else if x_max > t_max {
            IInterval::full(target)
        } else {
            IInterval::new_signed(target, self.min, self.max)
        }
    }
    /// Casts a signed interval to an unsigned one of a type with the same bit width.
    ///
    /// If the type is already unsigned, the result is unspecified.
    pub(crate) const fn cast_signed_to_unsigned(&self) -> Self {
        debug_assert!(self.ty.is_signed());

        let target = self.ty.swap_signedness();
        if self.is_empty() {
            return Self::empty(target);
        }

        let t_max = target.max_value().cast_unsigned();
        let (x_min, x_max) = self.as_signed();

        if x_max < 0 {
            IInterval::new_unsigned(target, x_min.cast_unsigned() & t_max, x_max.cast_unsigned() & t_max)
        } else if x_min < 0 {
            IInterval::full(target)
        } else {
            IInterval::new_unsigned(target, self.min.cast_unsigned(), self.max.cast_unsigned())
        }
    }
    /// Casts an interval to a different wider type of the same signedness.
    ///
    /// If the signedness of the target type is different or the target type is
    /// narrower, the result is unspecified.
    pub(crate) fn cast_widen(&self, target: IntType) -> Self {
        debug_assert!(self.ty.is_signed() == target.is_signed());
        debug_assert!(self.ty.bits() <= target.bits());

        let mut result = self.clone();
        result.ty = target;
        result
    }
}

impl std::fmt::Display for IInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "<empty>[{:?}]", self.ty)
        } else if self.ty.is_signed() {
            let (min, max) = self.as_signed();
            if min == max {
                write!(f, "{min}[{:?}]", self.ty)
            } else {
                write!(f, "{min}..={max}[{:?}]", self.ty)
            }
        } else {
            let (min, max) = self.as_unsigned();
            if min == max {
                write!(f, "{min}[{:?}]", self.ty)
            } else {
                write!(f, "{min}..={max}[{:?}]", self.ty)
            }
        }
    }
}
