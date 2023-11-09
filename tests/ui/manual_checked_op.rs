#![warn(clippy::manual_checked_op)]
#![allow(clippy::int_plus_one)]

fn main() {
    // checked_shl
    {
        let x = 1u32;
        let _ = if x < 8 { 1u8 << x } else { panic!() };
        let _ = if x <= 7 { 1u8 << x } else { panic!() };
        let _ = if x < 16 { 1u16 << x } else { panic!() };
        let _ = if x <= 15 { 1u16 << x } else { panic!() };
        let _ = if x < 32 { 1u32 << x } else { panic!() };
        let _ = if x <= 31 { 1u32 << x } else { panic!() };
        let _ = if x < 64 { 1u64 << x } else { panic!() };
        let _ = if x <= 63 { 1u64 << x } else { panic!() };
        let _ = if x < 128 { 1u128 << x } else { panic!() };
        let _ = if x <= 127 { 1u128 << x } else { panic!() };

        let _ = if x < u8::BITS { 1u8 << x } else { panic!() };
        let _ = if x < u16::BITS { 1u16 << x } else { panic!() };
        let _ = if x < u32::BITS { 1u32 << x } else { panic!() };
        let _ = if x < u64::BITS { 1u64 << x } else { panic!() };
        let _ = if x < u128::BITS { 1u128 << x } else { panic!() };
        let _ = if x < usize::BITS { 1usize << x } else { panic!() };
        let _ = if x <= usize::BITS - 1 { 1usize << x } else { panic!() };

        let _ = if x < 8 { 1i8 << x } else { panic!() };
        let _ = if x <= 7 { 1i8 << x } else { panic!() };
        let _ = if x < 16 { 1i16 << x } else { panic!() };
        let _ = if x <= 15 { 1i16 << x } else { panic!() };
        let _ = if x < 32 { 1i32 << x } else { panic!() };
        let _ = if x <= 31 { 1i32 << x } else { panic!() };
        let _ = if x < 64 { 1i64 << x } else { panic!() };
        let _ = if x <= 63 { 1i64 << x } else { panic!() };
        let _ = if x < 128 { 1i128 << x } else { panic!() };
        let _ = if x <= 127 { 1i128 << x } else { panic!() };

        let _ = if x < i8::BITS { 1i8 << x } else { panic!() };
        let _ = if x < i16::BITS { 1i16 << x } else { panic!() };
        let _ = if x < i32::BITS { 1i32 << x } else { panic!() };
        let _ = if x < i64::BITS { 1i64 << x } else { panic!() };
        let _ = if x < i128::BITS { 1i128 << x } else { panic!() };
        let _ = if x < isize::BITS { 1isize << x } else { panic!() };
        let _ = if x <= isize::BITS - 1 { 1isize << x } else { panic!() };

        // Off by 1
        let _ = if x < 9 { 1u8 << x } else { panic!() };
        let _ = if x <= 8 { 1u8 << x } else { panic!() };
        let _ = if x < 17 { 1u16 << x } else { panic!() };
        let _ = if x <= 16 { 1u16 << x } else { panic!() };
        let _ = if x < 33 { 1u32 << x } else { panic!() };
        let _ = if x <= 32 { 1u32 << x } else { panic!() };
        let _ = if x < 65 { 1u64 << x } else { panic!() };
        let _ = if x <= 64 { 1u64 << x } else { panic!() };
        let _ = if x < 129 { 1u128 << x } else { panic!() };
        let _ = if x <= 128 { 1u128 << x } else { panic!() };

        let _ = if x < 9 { 1i8 << x } else { panic!() };
        let _ = if x <= 8 { 1i8 << x } else { panic!() };
        let _ = if x < 17 { 1i16 << x } else { panic!() };
        let _ = if x <= 16 { 1i16 << x } else { panic!() };
        let _ = if x < 33 { 1i32 << x } else { panic!() };
        let _ = if x <= 32 { 1i32 << x } else { panic!() };
        let _ = if x < 65 { 1i64 << x } else { panic!() };
        let _ = if x <= 64 { 1i64 << x } else { panic!() };
        let _ = if x < 129 { 1i128 << x } else { panic!() };
        let _ = if x <= 128 { 1i128 << x } else { panic!() };

        let _ = if x < 7 { 1u8 << x } else { panic!() };
        let _ = if x <= 6 { 1u8 << x } else { panic!() };
        let _ = if x < 15 { 1u16 << x } else { panic!() };
        let _ = if x <= 14 { 1u16 << x } else { panic!() };
        let _ = if x < 31 { 1u32 << x } else { panic!() };
        let _ = if x <= 30 { 1u32 << x } else { panic!() };
        let _ = if x < 63 { 1u64 << x } else { panic!() };
        let _ = if x <= 62 { 1u64 << x } else { panic!() };
        let _ = if x < 127 { 1u128 << x } else { panic!() };
        let _ = if x <= 126 { 1u128 << x } else { panic!() };

        let _ = if x < 7 { 1i8 << x } else { panic!() };
        let _ = if x <= 6 { 1i8 << x } else { panic!() };
        let _ = if x < 15 { 1i16 << x } else { panic!() };
        let _ = if x <= 14 { 1i16 << x } else { panic!() };
        let _ = if x < 31 { 1i32 << x } else { panic!() };
        let _ = if x <= 30 { 1i32 << x } else { panic!() };
        let _ = if x < 63 { 1i64 << x } else { panic!() };
        let _ = if x <= 62 { 1i64 << x } else { panic!() };
        let _ = if x < 127 { 1i128 << x } else { panic!() };
        let _ = if x <= 126 { 1i128 << x } else { panic!() };

        // wrong usize / isize
        let _ = if x < 32 { 1usize << x } else { panic!() };
        let _ = if x < 64 { 1usize << x } else { panic!() };
        let _ = if x < 32 { 1isize << x } else { panic!() };
        let _ = if x < 64 { 1isize << x } else { panic!() };
    }

    // checked_shr
    {
        let x = 1u32;
        let _ = if x < 8 { 1u8 >> x } else { panic!() };
        let _ = if x <= 7 { 1u8 >> x } else { panic!() };
        let _ = if x < 16 { 1u16 >> x } else { panic!() };
        let _ = if x <= 15 { 1u16 >> x } else { panic!() };
        let _ = if x < 32 { 1u32 >> x } else { panic!() };
        let _ = if x <= 31 { 1u32 >> x } else { panic!() };
        let _ = if x < 64 { 1u64 >> x } else { panic!() };
        let _ = if x <= 63 { 1u64 >> x } else { panic!() };
        let _ = if x < 128 { 1u128 >> x } else { panic!() };
        let _ = if x <= 127 { 1u128 >> x } else { panic!() };

        let _ = if x < u8::BITS { 1u8 >> x } else { panic!() };
        let _ = if x < u16::BITS { 1u16 >> x } else { panic!() };
        let _ = if x < u32::BITS { 1u32 >> x } else { panic!() };
        let _ = if x < u64::BITS { 1u64 >> x } else { panic!() };
        let _ = if x < u128::BITS { 1u128 >> x } else { panic!() };
        let _ = if x < usize::BITS { 1usize >> x } else { panic!() };
        let _ = if x <= usize::BITS - 1 { 1usize >> x } else { panic!() };

        let _ = if x < 8 { 1i8 >> x } else { panic!() };
        let _ = if x <= 7 { 1i8 >> x } else { panic!() };
        let _ = if x < 16 { 1i16 >> x } else { panic!() };
        let _ = if x <= 15 { 1i16 >> x } else { panic!() };
        let _ = if x < 32 { 1i32 >> x } else { panic!() };
        let _ = if x <= 31 { 1i32 >> x } else { panic!() };
        let _ = if x < 64 { 1i64 >> x } else { panic!() };
        let _ = if x <= 63 { 1i64 >> x } else { panic!() };
        let _ = if x < 128 { 1i128 >> x } else { panic!() };
        let _ = if x <= 127 { 1i128 >> x } else { panic!() };

        let _ = if x < i8::BITS { 1i8 >> x } else { panic!() };
        let _ = if x < i16::BITS { 1i16 >> x } else { panic!() };
        let _ = if x < i32::BITS { 1i32 >> x } else { panic!() };
        let _ = if x < i64::BITS { 1i64 >> x } else { panic!() };
        let _ = if x < i128::BITS { 1i128 >> x } else { panic!() };
        let _ = if x < isize::BITS { 1isize >> x } else { panic!() };
        let _ = if x <= isize::BITS - 1 { 1isize >> x } else { panic!() };

        // Off by 1
        let _ = if x < 9 { 1u8 >> x } else { panic!() };
        let _ = if x <= 8 { 1u8 >> x } else { panic!() };
        let _ = if x < 17 { 1u16 >> x } else { panic!() };
        let _ = if x <= 16 { 1u16 >> x } else { panic!() };
        let _ = if x < 33 { 1u32 >> x } else { panic!() };
        let _ = if x <= 32 { 1u32 >> x } else { panic!() };
        let _ = if x < 65 { 1u64 >> x } else { panic!() };
        let _ = if x <= 64 { 1u64 >> x } else { panic!() };
        let _ = if x < 129 { 1u128 >> x } else { panic!() };
        let _ = if x <= 128 { 1u128 >> x } else { panic!() };

        let _ = if x < 9 { 1i8 >> x } else { panic!() };
        let _ = if x <= 8 { 1i8 >> x } else { panic!() };
        let _ = if x < 17 { 1i16 >> x } else { panic!() };
        let _ = if x <= 16 { 1i16 >> x } else { panic!() };
        let _ = if x < 33 { 1i32 >> x } else { panic!() };
        let _ = if x <= 32 { 1i32 >> x } else { panic!() };
        let _ = if x < 65 { 1i64 >> x } else { panic!() };
        let _ = if x <= 64 { 1i64 >> x } else { panic!() };
        let _ = if x < 129 { 1i128 >> x } else { panic!() };
        let _ = if x <= 128 { 1i128 >> x } else { panic!() };

        let _ = if x < 7 { 1u8 >> x } else { panic!() };
        let _ = if x <= 6 { 1u8 >> x } else { panic!() };
        let _ = if x < 15 { 1u16 >> x } else { panic!() };
        let _ = if x <= 14 { 1u16 >> x } else { panic!() };
        let _ = if x < 31 { 1u32 >> x } else { panic!() };
        let _ = if x <= 30 { 1u32 >> x } else { panic!() };
        let _ = if x < 63 { 1u64 >> x } else { panic!() };
        let _ = if x <= 62 { 1u64 >> x } else { panic!() };
        let _ = if x < 127 { 1u128 >> x } else { panic!() };
        let _ = if x <= 126 { 1u128 >> x } else { panic!() };

        let _ = if x < 7 { 1i8 >> x } else { panic!() };
        let _ = if x <= 6 { 1i8 >> x } else { panic!() };
        let _ = if x < 15 { 1i16 >> x } else { panic!() };
        let _ = if x <= 14 { 1i16 >> x } else { panic!() };
        let _ = if x < 31 { 1i32 >> x } else { panic!() };
        let _ = if x <= 30 { 1i32 >> x } else { panic!() };
        let _ = if x < 63 { 1i64 >> x } else { panic!() };
        let _ = if x <= 62 { 1i64 >> x } else { panic!() };
        let _ = if x < 127 { 1i128 >> x } else { panic!() };
        let _ = if x <= 126 { 1i128 >> x } else { panic!() };

        // wrong usize / isize
        let _ = if x < 32 { 1usize >> x } else { panic!() };
        let _ = if x < 64 { 1usize >> x } else { panic!() };
        let _ = if x < 32 { 1isize >> x } else { panic!() };
        let _ = if x < 64 { 1isize >> x } else { panic!() };
    }

    // alt forms
    {
        let x = 1;
        let _ = if x >= 8 { panic!() } else { 1u8 << x };
        let _ = if x < 8 { 1u8 << x } else { panic!("custom message") };
        let _ = if x < 8 {
            1u8 << x
        } else {
            panic!("{x} formatted message")
        };
        let _ = if x < 8 { 1u8 << x } else { std::process::abort() };
        let _ = if x < 8 { Some(1u8 << x) } else { std::process::abort() };
        let _ = if x < 8 { Some(1u8 << x) } else { None };
    }

    // checked add
    {
        let x = 1u8;
        let _ = if x != u8::MAX { x + 1 } else { panic!() };
        let x = 1u16;
        let _ = if x != u16::MAX { x + 1 } else { panic!() };
        let x = 1u32;
        let _ = if x != u32::MAX { x + 1 } else { panic!() };
        let x = 1u64;
        let _ = if x != u64::MAX { x + 1 } else { panic!() };
        let x = 1u128;
        let _ = if x != u128::MAX { x + 1 } else { panic!() };

        let x = 1i8;
        let _ = if x != i8::MAX { x + 1 } else { panic!() };
        let x = 1i16;
        let _ = if x != i16::MAX { x + 1 } else { panic!() };
        let x = 1i32;
        let _ = if x != i32::MAX { x + 1 } else { panic!() };
        let x = 1i64;
        let _ = if x != i64::MAX { x + 1 } else { panic!() };
        let x = 1i128;
        let _ = if x != i128::MAX { x + 1 } else { panic!() };

        let x = 1u8;
        let _ = if x < u8::MAX - 2 { x + 2 } else { panic!() };
        let x = 1u16;
        let _ = if x < u16::MAX - 2 { x + 2 } else { panic!() };
        let x = 1u32;
        let _ = if x < u32::MAX - 2 { x + 2 } else { panic!() };
        let x = 1u64;
        let _ = if x < u64::MAX - 2 { x + 2 } else { panic!() };
        let x = 1u128;
        let _ = if x < u128::MAX - 2 { x + 2 } else { panic!() };

        let x = 1i8;
        let _ = if x < i8::MAX - 2 { x + 2 } else { panic!() };
        let x = 1i16;
        let _ = if x < i16::MAX - 2 { x + 2 } else { panic!() };
        let x = 1i32;
        let _ = if x < i32::MAX - 2 { x + 2 } else { panic!() };
        let x = 1i64;
        let _ = if x < i64::MAX - 2 { x + 2 } else { panic!() };
        let x = 1i128;
        let _ = if x < i128::MAX - 2 { x + 2 } else { panic!() };

        let x = 1u8;
        let _ = if u8::MAX != x { 1 + x } else { panic!() };
        let _ = if u8::MAX - 3 > x { x + 3 } else { panic!() };
        let _ = if x < u8::MAX - 3 { 3 + x } else { panic!() };

        // Off by one
        let x = 1u8;
        let _ = if x < u8::MAX - 2 { x + 3 } else { panic!() };
        let x = 1u16;
        let _ = if x < u16::MAX - 2 { x + 3 } else { panic!() };
        let x = 1u32;
        let _ = if x < u32::MAX - 2 { x + 3 } else { panic!() };
        let x = 1u64;
        let _ = if x < u64::MAX - 2 { x + 3 } else { panic!() };
        let x = 1u128;
        let _ = if x < u128::MAX - 2 { x + 3 } else { panic!() };

        let x = 1u8;
        let _ = if x < u8::MAX - 2 { x + 1 } else { panic!() };
        let x = 1u16;
        let _ = if x < u16::MAX - 2 { x + 1 } else { panic!() };
        let x = 1u32;
        let _ = if x < u32::MAX - 2 { x + 1 } else { panic!() };
        let x = 1u64;
        let _ = if x < u64::MAX - 2 { x + 1 } else { panic!() };
        let x = 1u128;
        let _ = if x < u128::MAX - 2 { x + 1 } else { panic!() };

        let x = 1u8;
        let _ = if x != u8::MAX { x + 2 } else { panic!() };
    }
}
