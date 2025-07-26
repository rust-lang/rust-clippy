//@no-rustfix: only some diagnostics have suggestions

#![warn(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(
    clippy::cast_abs_to_unsigned,
    clippy::no_effect,
    clippy::unnecessary_min_or_max,
    clippy::unnecessary_operation,
    clippy::unnecessary_literal_unwrap,
    clippy::identity_op
)]

// FIXME(f16_f128): add tests once const casting is available for these types
fn get_value<T>() -> T { todo!() }
fn main() {
    // Test clippy::cast_precision_loss
    let x0: i32 = get_value();
    x0 as f32;
    //~^ cast_precision_loss

    let x1: i64 = get_value();
    x1 as f32;
    //~^ cast_precision_loss

    x1 as f64;
    //~^ cast_precision_loss

    let x2: u32 = get_value();
    x2 as f32;
    //~^ cast_precision_loss

    let x3: u64 = get_value();
    x3 as f32;
    //~^ cast_precision_loss

    x3 as f64;
    //~^ cast_precision_loss

    // Test clippy::cast_possible_truncation
    1f32 as i32;
    //~^ cast_possible_truncation

    1f32 as u32;
    //~^ cast_possible_truncation
    //~| cast_sign_loss

    1f64 as f32;
    //~^ cast_possible_truncation

    1i32 as i8;
    //~^ cast_possible_truncation

    1i32 as u8;
    //~^ cast_possible_truncation

    1f64 as isize;
    //~^ cast_possible_truncation

    1f64 as usize;
    //~^ cast_possible_truncation
    //~| cast_sign_loss

    1f32 as u32 as u16;
    //~^ cast_possible_truncation
    //~| cast_possible_truncation
    //~| cast_sign_loss

    {
        let _x: i8 = 1i32 as _;
        //~^ cast_possible_truncation

        1f32 as i32;
        //~^ cast_possible_truncation

        1f64 as i32;
        //~^ cast_possible_truncation

        1f32 as u8;
        //~^ cast_possible_truncation
        //~| cast_sign_loss
    }
    // Test clippy::cast_possible_wrap
    1u8 as i8;
    //~^ cast_possible_wrap

    1u16 as i16;
    //~^ cast_possible_wrap

    1u32 as i32;
    //~^ cast_possible_wrap

    1u64 as i64;
    //~^ cast_possible_wrap

    1usize as isize;
    //~^ cast_possible_wrap

    // should not wrap, usize is never 8 bits
    1usize as i8;
    //~^ cast_possible_truncation

    // wraps on 16 bit ptr size
    1usize as i16;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap

    // wraps on 32 bit ptr size
    1usize as i32;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap

    // wraps on 64 bit ptr size
    1usize as i64;
    //~^ cast_possible_wrap

    // should not wrap, isize is never 8 bits
    1u8 as isize;
    // wraps on 16 bit ptr size
    1u16 as isize;
    //~^ cast_possible_wrap

    // wraps on 32 bit ptr size
    1u32 as isize;
    //~^ cast_possible_wrap

    // wraps on 64 bit ptr size
    1u64 as isize;
    //~^ cast_possible_truncation
    //~| cast_possible_wrap

    // Test clippy::cast_sign_loss
    1i32 as u32;
    -1i32 as u32;
    //~^ cast_sign_loss

    1isize as usize;
    -1isize as usize;
    //~^ cast_sign_loss

    0i8 as u8;
    i8::MAX as u8;
    i16::MAX as u16;
    i32::MAX as u32;
    i64::MAX as u64;
    i128::MAX as u128;

    (-1i8).saturating_abs() as u8;
    // abs() can return a negative value in release builds
    (i8::MIN).abs() as u8;
    //~^ cast_sign_loss

    (-1i16).saturating_abs() as u16;
    (-1i32).saturating_abs() as u32;
    (-1i64).abs() as u64;
    //~^ cast_sign_loss
    (-1isize).abs() as usize;
    //~^ cast_sign_loss

    (-1i8).checked_abs().unwrap() as u8;
    (i8::MIN).checked_abs().unwrap() as u8;
    (-1i16).checked_abs().unwrap() as u16;
    (-1i32).checked_abs().unwrap() as u32;
    // SAFETY: -1 is a small number which will always return Some
    (unsafe { (-1i64).checked_abs().unwrap_unchecked() }) as u64;
    //~^ cast_sign_loss
    (-1isize).checked_abs().expect("-1 is a small number") as usize;

    (-1i8).isqrt() as u8;
    (i8::MIN).isqrt() as u8;
    (-1i16).isqrt() as u16;
    (-1i32).isqrt() as u32;
    (-1i64).isqrt() as u64;
    (-1isize).isqrt() as usize;

    (-1i8).checked_isqrt().unwrap() as u8;
    (i8::MIN).checked_isqrt().unwrap() as u8;
    (-1i16).checked_isqrt().unwrap() as u16;
    (-1i32).checked_isqrt().unwrap() as u32;
    // SAFETY: -1 is a small number which will always return Some
    (unsafe { (-1i64).checked_isqrt().unwrap_unchecked() }) as u64;
    //~^ cast_sign_loss
    (-1isize).checked_isqrt().expect("-1 is a small number") as usize;

    (-1i8).rem_euclid(1i8) as u8;
    (-1i8).wrapping_rem_euclid(1i8) as u16;
    (-1i16).rem_euclid(1i16) as u16;
    (-1i16).rem_euclid(1i16) as u32;
    (-1i32).rem_euclid(1i32) as u32;
    (-1i32).rem_euclid(1i32) as u64;
    (-1i64).rem_euclid(1i64) as u64;
    (-1i64).rem_euclid(1i64) as u128;
    (-1isize).rem_euclid(1isize) as usize;
    (1i8).rem_euclid(-1i8) as u8;
    (1i8).wrapping_rem_euclid(-1i8) as u16;
    (1i16).rem_euclid(-1i16) as u16;
    (1i16).rem_euclid(-1i16) as u32;
    (1i32).rem_euclid(-1i32) as u32;
    (1i32).rem_euclid(-1i32) as u64;
    (1i64).rem_euclid(-1i64) as u64;
    (1i64).rem_euclid(-1i64) as u128;
    (1isize).rem_euclid(-1isize) as usize;

    (-1i8).checked_rem_euclid(1i8).unwrap() as u8;
    (-1i8).checked_rem_euclid(1i8).unwrap() as u16;
    (-1i16).checked_rem_euclid(1i16).unwrap() as u16;
    (-1i16).checked_rem_euclid(1i16).unwrap() as u32;
    (-1i32).checked_rem_euclid(1i32).unwrap() as u32;
    (-1i32).checked_rem_euclid(1i32).unwrap() as u64;
    (-1i64).checked_rem_euclid(1i64).unwrap() as u64;
    (-1i64).checked_rem_euclid(1i64).unwrap() as u128;
    (-1isize).checked_rem_euclid(1isize).unwrap() as usize;
    (1i8).checked_rem_euclid(-1i8).unwrap() as u8;
    (1i8).checked_rem_euclid(-1i8).unwrap() as u16;
    (1i16).checked_rem_euclid(-1i16).unwrap() as u16;
    (1i16).checked_rem_euclid(-1i16).unwrap() as u32;
    (1i32).checked_rem_euclid(-1i32).unwrap() as u32;
    (1i32).checked_rem_euclid(-1i32).unwrap() as u64;
    (1i64).checked_rem_euclid(-1i64).unwrap() as u64;
    (1i64).checked_rem_euclid(-1i64).unwrap() as u128;
    (1isize).checked_rem_euclid(-1isize).unwrap() as usize;

    // no lint for `cast_possible_truncation`
    // with `signum` method call (see issue #5395)
    let x: i64 = 5;
    let _ = x.signum() as i32;

    let s = x.signum();
    let _ = s as i32;

    // Test for signed min
    // should be linted because signed
    (-99999999999i64).min(1) as i8;
    //~^ cast_possible_truncation

    // Test for various operations that remove enough bits for the result to fit
    (999999u64 & 1) as u8;
    (999999u64 % 15) as u8;
    (999999u64 / 0x1_0000_0000_0000) as u16;
    ({ 999999u64 >> 56 }) as u8;
    ({
        let x = 999999u64;
        x.min(1)
    }) as u8;
    999999u64.clamp(0, 255) as u8;
    // should still be linted
    999999u64.clamp(0, 256) as u8;
    //~^ cast_possible_truncation

    #[derive(Clone, Copy)]
    enum E1 {
        A,
        B,
        C,
    }
    impl E1 {
        fn test(self) {
            // Don't lint. `0..=2` fits in u8
            let _ = self as u8;
        }
    }

    #[derive(Clone, Copy)]
    enum E2 {
        A = 255,
        B,
    }
    impl E2 {
        fn test(self) {
            let _ = self as u8;
            //~^ cast_possible_truncation

            let _ = Self::B as u8;
            //~^ cast_enum_truncation

            // Don't lint. `255..=256` fits in i16
            let _ = self as i16;
            // Don't lint.
            let _ = Self::A as u8;
        }
    }

    #[derive(Clone, Copy)]
    enum E3 {
        A = -1,
        B,
        C = 50,
    }
    impl E3 {
        fn test(self) {
            // Don't lint. `-1..=50` fits in i8
            let _ = self as i8;
        }
    }

    #[derive(Clone, Copy)]
    enum E4 {
        A = -128,
        B,
    }
    impl E4 {
        fn test(self) {
            // Don't lint. `-128..=-127` fits in i8
            let _ = self as i8;
        }
    }

    #[derive(Clone, Copy)]
    enum E5 {
        A = -129,
        B = 127,
    }
    impl E5 {
        fn test(self) {
            let _ = self as i8;
            //~^ cast_possible_truncation

            let _ = Self::A as i8;
            //~^ cast_enum_truncation

            // Don't lint. `-129..=127` fits in i16
            let _ = self as i16;
            // Don't lint.
            let _ = Self::B as u8;
        }
    }

    #[derive(Clone, Copy)]
    #[repr(u32)]
    enum E6 {
        A = u16::MAX as u32,
        B,
    }
    impl E6 {
        fn test(self) {
            let _ = self as i16;
            //~^ cast_possible_truncation

            // Don't lint. `2^16-1` fits in u16
            let _ = Self::A as u16;
            // Don't lint. `2^16-1..=2^16` fits in u32
            let _ = self as u32;
            // Don't lint.
            let _ = Self::A as u16;
        }
    }

    #[derive(Clone, Copy)]
    #[repr(u64)]
    enum E7 {
        A = u32::MAX as u64,
        B,
    }
    impl E7 {
        fn test(self) {
            let _ = self as usize;
            //~^ cast_possible_truncation

            // Don't lint.
            let _ = Self::A as usize;
            // Don't lint. `2^32-1..=2^32` fits in u64
            let _ = self as u64;
        }
    }

    #[derive(Clone, Copy)]
    #[repr(i128)]
    enum E8 {
        A = i128::MIN,
        B,
        C = 0,
        D = i128::MAX,
    }
    impl E8 {
        fn test(self) {
            // Don't lint. `-(2^127)..=2^127-1` fits it i128
            let _ = self as i128;
        }
    }

    #[derive(Clone, Copy)]
    #[repr(u128)]
    enum E9 {
        A,
        B = u128::MAX,
    }
    impl E9 {
        fn test(self) {
            // Don't lint.
            let _ = Self::A as u8;
            // Don't lint. `0..=2^128-1` fits in u128
            let _ = self as u128;
        }
    }

    #[derive(Clone, Copy)]
    #[repr(usize)]
    enum E10 {
        A,
        B = u32::MAX as usize,
    }
    impl E10 {
        fn test(self) {
            let _ = self as u16;
            //~^ cast_possible_truncation

            // Don't lint.
            let _ = Self::B as u32;
            // Don't lint.
            let _ = self as u64;
        }
    }
}

fn avoid_subtract_overflow(q: u32) {
    let c = (q >> 16) as u8;
    //~^ cast_possible_truncation

    c as usize;

    let c = (q / 1000) as u8;
    //~^ cast_possible_truncation

    c as usize;
}

fn issue11426() {
    (&42u8 >> 0xa9008fb6c9d81e42_0e25730562a601c8_u128) as usize;
}

fn issue11642() {
    fn square(x: i16) -> u32 {
        let x = x as i32;
        (x * x) as u32;
        //~^ cast_sign_loss
        x.pow(2) as u32;
        (-2_i32).saturating_pow(2) as u32
    }

    let _a = |x: i32| -> u32 { (x * x * x * x) as u32 };
    //~^ cast_sign_loss

    (2_i32).checked_pow(3).unwrap() as u32;
    //~^ cast_sign_loss
    (-2_i32).pow(3) as u32;
    //~^ cast_sign_loss

    (3_i32 % 2) as u32;
    (3_i32 % -2) as u32;
    (-5_i32 % 2) as u32;
    //~^ cast_sign_loss

    (-5_i32 % -2) as u32;
    //~^ cast_sign_loss

    (2_i32 >> 1) as u32;
    (-2_i32 >> 1) as u32;
    //~^ cast_sign_loss

    let x: i32 = get_value();
    (x * x) as u32;
    //~^ cast_sign_loss
    (x * x * x) as u32;
    //~^ cast_sign_loss

    let y: i16 = get_value();
    (y * y * y * y * -2) as u16;
    //~^ cast_sign_loss

    (y * y * y / y * 2) as u16;
    //~^ cast_sign_loss
    (y * y / y * 2) as u16;
    //~^ cast_sign_loss

    (y / y * y * -2) as u16;
    //~^ cast_sign_loss
    //~| eq_op

    (y + y + y + -2) as u16;
    //~^ cast_sign_loss

    (y + y + y + 2) as u16;
    //~^ cast_sign_loss

    let z: i16 = get_value();
    (z + -2) as u16;
    //~^ cast_sign_loss

    (z + z + 2) as u16;
    //~^ cast_sign_loss

    fn foo(a: i32, b: i32, c: i32) -> u32 {
        (a * a * b * b * c * c) as u32;
        //~^ cast_sign_loss
        (a * b * c) as u32;
        //~^ cast_sign_loss

        (a * -b * c) as u32;
        //~^ cast_sign_loss

        (a * b * c * c) as u32;
        //~^ cast_sign_loss
        (a * -2) as u32;
        //~^ cast_sign_loss

        (a * b * c * -2) as u32;
        //~^ cast_sign_loss

        (a / b) as u32;
        //~^ cast_sign_loss
        (a / b * c) as u32;
        //~^ cast_sign_loss

        (a / b + b * c) as u32;
        //~^ cast_sign_loss

        a.saturating_pow(3) as u32;
        //~^ cast_sign_loss

        (a.abs() * b.pow(2) / c.abs()) as u32
        //~^ cast_sign_loss
    }
}

fn issue11738() {
    macro_rules! m {
        () => {
            let _ = i32::MIN as u32; // cast_sign_loss
            //
            //~^^ cast_sign_loss
            let _ = u32::MAX as u8; // cast_possible_truncation
            //
            //~^^ cast_possible_truncation
            let _ = std::f64::consts::PI as f32; // cast_possible_truncation
            //
            //~^^ cast_possible_truncation
            let _ = 0i8 as i32; // cast_lossless
        };
    }
    m!();
}

fn issue12506() -> usize {
    let bar: Result<Option<i64>, u32> = Ok(Some(10));
    bar.unwrap().unwrap() as usize
    //~^ cast_possible_truncation
    //~| cast_sign_loss
}

fn issue12721() {
    fn x() -> u64 {
        u64::MAX
    }

    // Don't lint.
    (255 & 999999u64) as u8;
    // Don't lint.
    let _ = ((x() & 255) & 999999) as u8;
    // Don't lint.
    let _ = (999999 & (x() & 255)) as u8;

    (256 & 999999u64) as u8;
    //~^ cast_possible_truncation

    (255 % 999999u64) as u8;
    //~^ cast_possible_truncation
}

fn issue7486(number: u64, other: u32) -> u32 {
    // a u64 with guaranteed value to be less than or equal to u32::max_value()
    let other_u64 = other as u64;
    // modulo guarantees that the following invariant holds:
    // result < other_u64
    // which implies that result < u32::max_value()
    let result = number % other_u64;
    result as u32
    //~^ cast_possible_truncation
}

fn dxgi_xr10_to_unorm8(x: u16) -> u8 {
    debug_assert!(x <= 1023);
    // new range: [-384, 639] (or [-0.75294, 1.25294])
    let x = x as i16 - 0x180;
    //~^ cast_possible_wrap
    // new range: [0, 510] (or [0.0, 1.0])
    let x = x.clamp(0, 510) as u16;
    //~^ cast_sign_loss
    // this is round(x / 510 * 255), but faster
    ((x + 1) >> 1) as u8
    //~^ cast_possible_truncation
}
fn dxgi_xr10_to_unorm16(x: u16) -> u16 {
    debug_assert!(x <= 1023);
    // new range: [-384, 639] (or [-0.75294, 1.25294])
    let x = x as i16 - 0x180;
    //~^ cast_possible_wrap
    // new range: [0, 510] (or [0.0, 1.0])
    let x = x.clamp(0, 510) as u16;
    //~^ cast_sign_loss
    // this is round(x / 510 * 65535), but faster
    ((x as u32 * 8421376 + 65535) >> 16) as u16
}
fn unorm16_to_unorm8(x: u16) -> u8 {
    ((x as u32 * 255 + 32895) >> 16) as u8
    //~^ cast_possible_truncation
}

fn f32_to_f16u(value: f32) -> u16 {
    // Source: https://github.com/starkat99/half-rs/blob/2c4122db4e8f7d8ce030bb4b5ed8913bd6bbf2b1/src/binary16/arch.rs#L482
    // Author: Kathryn Long
    // License: MIT OR Apache-2.0

    // Convert to raw bytes
    let x: u32 = value.to_bits();

    // Extract IEEE754 components
    let sign = x & 0x8000_0000u32;
    let exp = x & 0x7F80_0000u32;
    let man = x & 0x007F_FFFFu32;

    // Check for all exponent bits being set, which is Infinity or NaN
    if exp == 0x7F80_0000u32 {
        // Set mantissa MSB for NaN (and also keep shifted mantissa bits)
        let nan_bit = if man == 0 { 0 } else { 0x0200u32 };
        return ((sign >> 16) | 0x7C00u32 | nan_bit | (man >> 13)) as u16;
        //~^ cast_possible_truncation
    }

    // The number is normalized, start assembling half precision version
    let half_sign = sign >> 16;
    // Unbias the exponent, then bias for half precision
    let unbiased_exp = ((exp >> 23) as i32) - 127;
    //~^ cast_possible_wrap
    let half_exp = unbiased_exp + 15;

    // Check for exponent overflow, return +infinity
    if half_exp >= 0x1F {
        return (half_sign | 0x7C00u32) as u16;
        //~^ cast_possible_truncation
    }

    // Check for underflow
    if half_exp <= 0 {
        // Check mantissa for what we can do
        if 14 - half_exp > 24 {
            // No rounding possibility, so this is a full underflow, return signed zero
            return half_sign as u16;
        }
        // Don't forget about hidden leading mantissa bit when assembling mantissa
        let man = man | 0x0080_0000u32;
        let mut half_man = man >> (14 - half_exp);
        // Check for rounding (see comment above functions)
        let round_bit = 1 << (13 - half_exp);
        if (man & round_bit) != 0 && (man & (3 * round_bit - 1)) != 0 {
            half_man += 1;
        }
        // No exponent for subnormals
        return (half_sign | half_man) as u16;
        //~^ cast_possible_truncation
    }

    // Rebias the exponent
    let half_exp = (half_exp as u32) << 10;
    //~^ cast_sign_loss
    let half_man = man >> 13;
    // Check for rounding (see comment above functions)
    let round_bit = 0x0000_1000u32;
    if (man & round_bit) != 0 && (man & (3 * round_bit - 1)) != 0 {
        // Round it
        ((half_sign | half_exp | half_man) + 1) as u16
        //~^ cast_possible_truncation
    } else {
        (half_sign | half_exp | half_man) as u16
        //~^ cast_possible_truncation
    }
}
