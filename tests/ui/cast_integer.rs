#![warn(clippy::cast_integer)]

fn main() {
    let u = 1usize;
    let _ = u as u64;
    let _ = u64::try_from(u);

    // unsigned
    let u64= 1u64;
    let _ = u64 as u128;
    let _ = u128::from(u64);

    let u32 = 1u32;
    let _ = u32 as u128;
    let _ = u128::from(u32);
    let _ = u32 as u64;
    let _ = u64::from(u32);

    let u16= 1u16;
    let _ = u16 as u128;
    let _ = u128::from(u16);
    let _ = u16 as u64;
    let _ = u64::from(u16);
    let _ = u16 as u32;
    let _ = u32::from(u16);

    let u8 = 1u8;
    let _ = u8 as u128;
    let _ = u128::from(u8);
    let _ = u8 as u64;
    let _ = u64::from(u8);
    let _ = u8 as u32;
    let _ = u32::from(u8);
    let _ = u8 as u16;
    let _ = u16::from(u8);

    // signed

    let isize= 1isize;
    let _ = isize as i64;
    let _ = i64::try_from(isize);

    let i64 = 1i64;
    let _ = i64 as i128;
    let _ = i128::from(i64);

    let i32 = 1i32;
    let _ = i32 as i128;
    let _ = i128::from(i32);
    let _ = i32 as i64;
    let _ = i64::from(i32);

    let i16 = 1i16;
    let _ = i16 as i128;
    let _ = i128::from(i16);
    let _ = i16 as i64;
    let _ = i64::from(i16);
    let _ = i16 as i32;
    let _ = i32::from(i16);

    let i8 = 1i8;
    let _ = i8 as i128;
    let _ = i128::from(i8);
    let _ = i8 as i64;
    let _ = i64::from(i8);
    let _ = i8 as i32;
    let _ = i32::from(i8);
    let _ = i8 as i16;
    let _ = i16::from(i8);
}

// The lint would suggest using `u32::from(input)` here but the `XX::from` function is not const,
// so we skip the lint if the expression is in a const fn.
// See #3656
const fn const_function(input: u32) -> u64 {
    input as u64
}

// Same as the above issue. We can't suggest `::from` in const fns in impls
mod cast_integer_in_impl {
    struct A;

    impl A {
        pub const fn convert(x: u32) -> u64 {
            x as u64
        }
    }
}