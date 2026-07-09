use core::iter;
use core::num::NonZero;
use rustc_ast::ast::{FloatTy, LitFloatType, LitIntType, LitKind};
use rustc_ast::token;
use rustc_span::{BytePos, Span, SpanData, Symbol};

use crate::sym;

/// Represents the base of a numeric literal, used for parsing and formatting.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Radix {
    /// A decimal literal (e.g., `123`)
    Dec,
    /// A binary literal (e.g., `0b1010`)
    Bin,
    /// An octal literal (e.g., `0o670`)
    Oct,
    /// A hexadecimal literal (e.g., `0xFF`)
    Hex,
}

impl Radix {
    /// Returns a reasonable digit group size for this radix.
    #[must_use]
    fn suggest_grouping(self) -> usize {
        match self {
            Self::Bin | Self::Hex => 4,
            Self::Oct | Self::Dec => 3,
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Self::Dec => "",
            Self::Bin => "0b",
            Self::Oct => "0o",
            Self::Hex => "0x",
        }
    }
}

/// A helper method to format numeric literals with digit grouping.
/// `lit` must be a valid numeric literal without suffix.
pub fn format(lit: &str, type_suffix: Option<&str>, float: bool) -> String {
    NumericLiteral::new(lit, type_suffix, float).format()
}

#[derive(Debug)]
pub struct NumericLiteral<'a> {
    /// Which radix the literal was represented in.
    pub radix: Radix,
    /// The radix prefix, if present.
    pub prefix: Option<&'a str>,

    /// The integer part of the number.
    pub integer: &'a str,
    /// The fraction part of the number.
    pub fraction: Option<&'a str>,
    /// The exponent separator (b'e' or b'E') including preceding underscore if present
    /// and the exponent part.
    pub exponent: Option<(&'a str, &'a str)>,

    /// The type suffix, including preceding underscore if present.
    pub suffix: Option<&'a str>,
}

impl<'a> NumericLiteral<'a> {
    /// Attempts to parse a `NumericLiteral` from the source string of an `ast::LitKind`.
    pub fn from_lit_kind(src: &'a str, lit_kind: &LitKind) -> Option<NumericLiteral<'a>> {
        let unsigned_src = src.strip_prefix('-').map_or(src, |s| s);
        if lit_kind.is_numeric()
            && unsigned_src
                .trim_start()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            let (unsuffixed, suffix) = split_suffix(src, lit_kind);
            let float = matches!(lit_kind, LitKind::Float(..));
            Some(NumericLiteral::new(unsuffixed, suffix, float))
        } else {
            None
        }
    }

    /// Parses a raw numeric literal string into its structured `NumericLiteral` parts.
    #[must_use]
    pub fn new(lit: &'a str, suffix: Option<&'a str>, float: bool) -> Self {
        let unsigned_lit = lit.trim_start_matches('-');
        // Determine delimiter for radix prefix, if present, and radix.
        let radix = if unsigned_lit.starts_with("0x") {
            Radix::Hex
        } else if unsigned_lit.starts_with("0b") {
            Radix::Bin
        } else if unsigned_lit.starts_with("0o") {
            Radix::Oct
        } else {
            Radix::Dec
        };

        // Grab part of the literal after prefix, if present.
        let (prefix, mut sans_prefix) = if radix == Radix::Dec {
            (None, lit)
        } else {
            let (p, s) = lit.split_at(2);
            (Some(p), s)
        };

        if suffix.is_some() && sans_prefix.ends_with('_') {
            // The '_' before the suffix isn't part of the digits
            sans_prefix = &sans_prefix[..sans_prefix.len() - 1];
        }

        let (integer, fraction, exponent) = Self::split_digit_parts(sans_prefix, float);

        Self {
            radix,
            prefix,
            integer,
            fraction,
            exponent,
            suffix,
        }
    }

    /// Checks if the literal's radix is `Radix::Decimal`
    pub fn is_decimal(&self) -> bool {
        self.radix == Radix::Dec
    }

    fn split_digit_parts(digits: &str, float: bool) -> (&str, Option<&str>, Option<(&str, &str)>) {
        let mut integer = digits;
        let mut fraction = None;
        let mut exponent = None;

        if float {
            for (i, c) in digits.char_indices() {
                match c {
                    '.' => {
                        integer = &digits[..i];
                        fraction = Some(&digits[i + 1..]);
                    },
                    'e' | 'E' => {
                        let exp_start = if digits[..i].ends_with('_') { i - 1 } else { i };

                        if integer.len() > exp_start {
                            integer = &digits[..exp_start];
                        } else {
                            fraction = Some(&digits[integer.len() + 1..exp_start]);
                        }
                        exponent = Some((&digits[exp_start..=i], &digits[i + 1..]));
                        break;
                    },
                    _ => {},
                }
            }
        }

        (integer, fraction, exponent)
    }

    /// Returns literal formatted in a sensible way.
    pub fn format(&self) -> String {
        let mut output = String::new();

        if let Some(prefix) = self.prefix {
            output.push_str(prefix);
        }

        let group_size = self.radix.suggest_grouping();

        Self::group_digits(&mut output, self.integer, group_size, true, self.radix == Radix::Hex);

        if let Some(fraction) = self.fraction {
            output.push('.');
            Self::group_digits(&mut output, fraction, group_size, false, false);
        }

        if let Some((separator, exponent)) = self.exponent {
            if !exponent.is_empty() && exponent != "0" {
                output.push_str(separator);
                Self::group_digits(&mut output, exponent, group_size, true, false);
            } else if exponent == "0" && self.fraction.is_none() && self.suffix.is_none() {
                output.push_str(".0");
            }
        }

        if let Some(suffix) = self.suffix {
            if output.ends_with('.') {
                output.push('0');
            }
            output.push('_');
            output.push_str(suffix);
        }

        output
    }

    fn group_digits(output: &mut String, input: &str, group_size: usize, partial_group_first: bool, zero_pad: bool) {
        debug_assert!(group_size > 0);

        let mut digits = input.chars().filter(|&c| c != '_');

        // The exponent may have a sign, output it early, otherwise it will be
        // treated as a digit
        if digits.clone().next() == Some('-') {
            let _: Option<char> = digits.next();
            output.push('-');
        }

        let first_group_size;

        if partial_group_first {
            first_group_size = (digits.clone().count() - 1) % group_size + 1;
            if zero_pad {
                for _ in 0..group_size - first_group_size {
                    output.push('0');
                }
            }
        } else {
            first_group_size = group_size;
        }

        for _ in 0..first_group_size {
            if let Some(digit) = digits.next() {
                output.push(digit);
            }
        }

        for (c, i) in iter::zip(digits, (0..group_size).cycle()) {
            if i == 0 {
                output.push('_');
            }
            output.push(c);
        }
    }
}

fn split_suffix<'a>(src: &'a str, lit_kind: &LitKind) -> (&'a str, Option<&'a str>) {
    debug_assert!(lit_kind.is_numeric());
    lit_suffix_length(lit_kind)
        .and_then(|suffix_length| src.len().checked_sub(suffix_length))
        .map_or((src, None), |split_pos| {
            let (unsuffixed, suffix) = src.split_at(split_pos);
            (unsuffixed, Some(suffix))
        })
}

fn lit_suffix_length(lit_kind: &LitKind) -> Option<usize> {
    debug_assert!(lit_kind.is_numeric());
    let suffix = match lit_kind {
        LitKind::Int(_, int_lit_kind) => match int_lit_kind {
            LitIntType::Signed(int_ty) => Some(int_ty.name_str()),
            LitIntType::Unsigned(uint_ty) => Some(uint_ty.name_str()),
            LitIntType::Unsuffixed => None,
        },
        LitKind::Float(_, float_lit_kind) => match float_lit_kind {
            LitFloatType::Suffixed(float_ty) => Some(float_ty.name_str()),
            LitFloatType::Unsuffixed => None,
        },
        _ => None,
    };

    suffix.map(str::len)
}

#[derive(Clone, Copy)]
pub enum NumTy {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    F16,
    F32,
    F64,
    F128,
}
impl NumTy {
    pub fn from_sym(s: Symbol) -> Option<Self> {
        match s {
            sym::u8 => Some(Self::U8),
            sym::u16 => Some(Self::U16),
            sym::u32 => Some(Self::U32),
            sym::u64 => Some(Self::U64),
            sym::u128 => Some(Self::U128),
            sym::usize => Some(Self::Usize),
            sym::i8 => Some(Self::I8),
            sym::i16 => Some(Self::I16),
            sym::i32 => Some(Self::I32),
            sym::i64 => Some(Self::I64),
            sym::i128 => Some(Self::I128),
            sym::isize => Some(Self::Isize),
            sym::f16 => Some(Self::F16),
            sym::f32 => Some(Self::F32),
            sym::f64 => Some(Self::F64),
            sym::f128 => Some(Self::F128),
            _ => None,
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::F16 | Self::F32 | Self::F64 | Self::F128)
    }

    pub fn name_str(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::F16 => "f16",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::F128 => "f128",
        }
    }
}

pub fn float_ty_from_sym(sym: Symbol) -> Option<FloatTy> {
    match sym {
        sym::f16 => Some(FloatTy::F16),
        sym::f32 => Some(FloatTy::F32),
        sym::f64 => Some(FloatTy::F64),
        sym::f128 => Some(FloatTy::F128),
        _ => None,
    }
}

/// The text of a floating point literal.
#[derive(Clone, Copy)]
pub struct FloatStr<'a> {
    /// The literal without the suffix separator.
    pub text: &'a str,
    pub dot: Option<NonZero<u32>>,
    pub exp: Option<NonZero<u32>>,
    has_exp_sign: bool,
    has_suffix_sep: bool,
    pub suffix: Option<FloatTy>,
}
impl<'a> FloatStr<'a> {
    #[expect(clippy::cast_possible_truncation)]
    pub fn from_token_lit(lit: &'a token::Lit) -> Option<Self> {
        let text = lit.symbol.as_str();
        let (mut dot, mut exp) = (0u32, 0u32);
        let mut has_exp_sign = false;
        let mut iter = text.bytes().enumerate();
        while let Some((i, c)) = iter.next() {
            match c {
                b'.' => dot = i as u32,
                b'e' | b'E' => {
                    exp = i as u32;
                    if let Some((_, b'+' | b'-')) = iter.clone().next() {
                        iter.next();
                        has_exp_sign = true;
                    }
                },
                _ => {},
            }
        }
        let (text, has_suffix_sep, suffix) = match lit.suffix {
            Some(suffix) => match float_ty_from_sym(suffix) {
                Some(suffix) => match text.strip_suffix('_') {
                    Some(text) => (text, true, Some(suffix)),
                    None => (text, false, Some(suffix)),
                },
                None => return None,
            },
            None => (text, false, None),
        };
        Some(Self {
            text,
            dot: NonZero::new(dot),
            exp: NonZero::new(exp),
            has_exp_sign,
            has_suffix_sep,
            suffix,
        })
    }

    pub fn eq_str(&self, s: &str) -> bool {
        let Some(s) = s.strip_prefix(self.text) else {
            return false;
        };
        let s = match (self.has_suffix_sep, s.as_bytes()) {
            (true, [b'_', s @ ..]) => s,
            (true, _) => return false,
            (false, s) => s,
        };
        s == self.suffix.map_or("", FloatTy::name_str).as_bytes()
    }

    pub fn exp_digits_str(&self) -> Option<&str> {
        self.exp
            .and_then(|pos| self.text.get(pos.get() as usize + 1 + usize::from(self.has_exp_sign)..))
    }

    /// Trims the span to just the digits of the exponent.
    ///
    /// This assumes the text of the span matches the literal.
    pub fn trim_sp_to_exp_digits(&self, sp: &SpanData) -> Span {
        Span::new(
            BytePos(sp.lo.0 + self.exp.map_or(0, NonZero::get) + 1 + u32::from(self.has_exp_sign)),
            BytePos(sp.lo.0 + self.text.len() as u32),
            sp.ctxt,
            sp.parent,
        )
    }

    /// Removes both the suffix the exponent from the span.
    ///
    /// This assumes the text of the span matches the literal.
    pub fn strip_exp_from_sp(&self, sp: &SpanData) -> Span {
        Span::new(
            sp.lo,
            BytePos(match self.exp {
                Some(exp) => {
                    sp.lo.0 + exp.get()
                        - u32::from(matches!(self.text.as_bytes().get(exp.get() as usize - 1), Some(b'_')))
                },
                None => sp.hi.0 - u32::from(self.has_suffix_sep) - self.suffix.map_or(0, |x| x.name_str().len() as u32),
            }),
            sp.ctxt,
            sp.parent,
        )
    }

    /// Gets the text for the whole part and the fractional part of the literal.
    pub fn whole_fract_digits_str(&self) -> (&'a str, Option<&'a str>) {
        let end = match self.exp {
            Some(exp) => {
                exp.get() as usize - usize::from(matches!(self.text.as_bytes().get(exp.get() as usize - 1), Some(b'_')))
            },
            None => self.text.len(),
        };
        if let Some(dot) = self.dot {
            (
                self.text.get(..dot.get() as usize).unwrap_or(self.text),
                self.text.get(dot.get() as usize + 1..end),
            )
        } else {
            (self.text.get(..end).unwrap_or(self.text), None)
        }
    }
}

#[derive(Clone, Copy)]
pub struct IntStr<'a> {
    /// The literal text without the prefix or suffix separator.
    pub text: &'a str,
    pub radix: Radix,
    pub has_suffix_sep: bool,
    pub suffix: Option<NumTy>,
}
impl<'a> IntStr<'a> {
    pub fn from_token_lit(lit: &'a token::Lit) -> Option<Self> {
        let text = lit.symbol.as_str();
        let (radix, text) = match text.as_bytes() {
            [b'0', b'x', ..] => (Radix::Hex, &text[2..]),
            [b'0', b'o', ..] => (Radix::Oct, &text[2..]),
            [b'0', b'b', ..] => (Radix::Bin, &text[2..]),
            _ => (Radix::Dec, text),
        };
        let (text, has_suffix_sep, suffix) = match lit.suffix {
            Some(suffix) => match NumTy::from_sym(suffix) {
                Some(suffix) => match text.strip_suffix('_') {
                    Some(text) => (text, true, Some(suffix)),
                    None => (text, false, Some(suffix)),
                },
                None => return None,
            },
            None => (text, false, None),
        };
        Some(Self {
            text,
            radix,
            has_suffix_sep,
            suffix,
        })
    }

    pub fn trim_sp_to_digits(&self, sp: &SpanData) -> Span {
        Span::new(
            BytePos(sp.lo.0 + self.radix.prefix().len() as u32),
            BytePos(sp.hi.0 - u32::from(self.has_suffix_sep) - self.suffix.map_or(0, |s| s.name_str().len() as u32)),
            sp.ctxt,
            sp.parent,
        )
    }

    /// Checks whether this literal's text exactly equals the unparsed text.
    pub fn eq_str(&self, s: &str) -> bool {
        if let Some(s) = s.strip_prefix(self.radix.prefix())
            && let Some(s) = s.strip_prefix(self.text)
        {
            let s = match (self.has_suffix_sep, s.as_bytes()) {
                (true, [b'_', s @ ..]) => s,
                (true, _) => return false,
                (false, s) => s,
            };
            s == self.suffix.map_or("", NumTy::name_str).as_bytes()
        } else {
            false
        }
    }

    /// Reformats the digits of this literal with the default digit grouping.
    pub fn reformat_digits(&self) -> String {
        // Allocate enough room to hold the formatted literal.
        let mut sugg = Vec::with_capacity(self.text.len() + self.text.len() / 3);
        let size = match self.radix {
            Radix::Bin | Radix::Hex => 4u8,
            Radix::Oct | Radix::Dec => 3u8,
        };
        reformat_int_rev_into(size, self.text, &mut sugg);
        sugg.reverse();
        String::from_utf8(sugg).unwrap_or_default()
    }
}

macro_rules! impl_parse_int_as {
    ($($ty:ident)*) => {
        impl IntStr<'_> {$(
            /// Parses the integer's digits.
            pub fn ${concat(parse_as_, $ty)}(&self) -> Option<$ty> {
                let mut acc: $ty = 0;
                let base_lg2 = match self.radix {
                    Radix::Dec => {
                        for c in self.text.bytes() {
                            let next = acc.checked_mul(10);
                            let c = match c {
                                b'_' => continue,
                                c => c - b'0',
                            };
                            acc = next?.checked_add($ty::from(c))?;
                        }
                        return Some(acc);
                    },
                    Radix::Bin => 1,
                    Radix::Oct => 2,
                    Radix::Hex => 4,
                };

                let limit = $ty::MAX >> base_lg2;
                for c in self.text.bytes() {
                    if acc > limit {
                        return None;
                    }
                    let c = match c {
                        b'0'..=b'9' => c - b'0',
                        b'A'..=b'F' => c - b'A',
                        b'_' => continue,
                        c => c - b'a',
                    };
                    acc = (acc << base_lg2) | $ty::from(c);
                }
                Some(acc)
            }
        )*}
    }
}
impl_parse_int_as!(u64 u128);

/// Reformats a float literal using the default digit grouping.
pub fn reformat_whole_opt_fract(whole: &str, fract: Option<&str>) -> String {
    // Allocate with enough room to hold the formatted literal.
    let cap = if let Some(fract) = fract {
        let digits = whole.len() + fract.len();
        digits + digits / 3 + 1
    } else {
        whole.len() + whole.len() / 3
    };
    let mut sugg = Vec::with_capacity(cap);
    reformat_int_rev_into(3, whole, &mut sugg);
    sugg.reverse();
    if let Some(fract) = fract {
        let _ = sugg.push_within_capacity(b'.');
        reformat_fract_into(fract, &mut sugg);
    }
    String::from_utf8(sugg).unwrap_or_default()
}

pub fn reformat_fract(fract: &str) -> String {
    // Allocate with enough room to hold the formatted literal.
    let mut sugg = Vec::with_capacity(fract.len() + fract.len() / 3);
    reformat_fract_into(fract, &mut sugg);
    String::from_utf8(sugg).unwrap_or_default()
}

fn reformat_int_rev_into(mut group_size: u8, num: &str, dst: &mut Vec<u8>) {
    let mut i = group_size;
    group_size -= 1;
    for c in num.bytes().rev() {
        if c != b'_' {
            if let Some(x) = i.checked_sub(1) {
                i = x;
            } else {
                let _ = dst.push_within_capacity(b'_');
                i = group_size;
            }
            let _ = dst.push_within_capacity(c);
        }
    }
}

fn reformat_fract_into(num: &str, dst: &mut Vec<u8>) {
    let mut i = 3u8;
    for c in num.bytes() {
        if c != b'_' {
            if let Some(x) = i.checked_sub(1) {
                i = x;
            } else {
                let _ = dst.push_within_capacity(b'_');
                i = 2;
            }
            let _ = dst.push_within_capacity(c);
        }
    }
}
