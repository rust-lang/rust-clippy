#![warn(clippy::manual_highest_one)]

use std::hint::black_box;
use std::num::NonZeroU32;

// This macro is transparent because the span of `$e` is not updated during expansion.
macro_rules! identity {
    ( $e:expr ) => {
        $e
    };
}

macro_rules! one {
    () => {
        1
    };
}
macro_rules! thirty_one {
    () => {
        31
    };
}
macro_rules! double {
    ( $i:ident ) => {
        2 * $i
    };
}

fn main() {
    let u = 5_u32;
    let i = -5_i32;
    let nz = NonZeroU32::new(5).unwrap();

    // --- Integer type ---
    #[allow(clippy::manual_ilog2, reason = "This pattern is shared.")]
    let _ = 31 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - 1 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - u.leading_zeros() - 1; //~ manual_highest_one
    let _ = u32::BITS - (1 + u.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS - (u.leading_zeros() + 1); //~ manual_highest_one
    let _ = u.bit_width() - 1; //~ manual_highest_one

    let _ = 31_u32.checked_sub(u.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS.checked_sub(u.leading_zeros() + 1); //~ manual_highest_one
    let _ = (u32::BITS - 1).checked_sub(u.leading_zeros()); //~ manual_highest_one
    let _ = u.bit_width().checked_sub(1); //~ manual_highest_one

    let _ = 31 - i.leading_zeros(); //~ manual_highest_one
    let _ = i32::BITS - 1 - i.leading_zeros(); //~ manual_highest_one
    let _ = i32::BITS - i.leading_zeros() - 1; //~ manual_highest_one
    let _ = i32::BITS - (1 + i.leading_zeros()); //~ manual_highest_one
    let _ = i32::BITS - (i.leading_zeros() + 1); //~ manual_highest_one

    // --- Nonzero type ---
    let _ = 31 - nz.leading_zeros(); //~ manual_highest_one
    let _ = NonZeroU32::BITS - 1 - nz.leading_zeros(); //~ manual_highest_one
    let _ = NonZeroU32::BITS - (1 + nz.leading_zeros()); //~ manual_highest_one
    let _ = NonZeroU32::BITS - (nz.leading_zeros() + 1); //~ manual_highest_one
    let _ = nz.bit_width().get() - 1; //~ manual_highest_one

    // --- In if block ---
    let _ = if u == 0 {
        //~^ manual_highest_one
        todo!()
    } else {
        u.bit_width() - 1
    };
    let _ = if 0 == u {
        //~^ manual_highest_one
        todo!()
    } else {
        u.bit_width() - 1
    };
    let _ = if u != 0 {
        //~^ manual_highest_one
        u.bit_width() - 1
    } else {
        todo!()
    };
    let _ = if u > 0 {
        //~^ manual_highest_one
        u.bit_width() - 1
    } else {
        todo!()
    };
    let _ = if i > 0 {
        31 - i.leading_zeros() //~ manual_highest_one
    } else {
        todo!()
    };

    // This should not be linted because ..
    let _ = if u == 0 {
        todo!()
    } else {
        // this arm may change state
        black_box(u);
        u.bit_width() - 1 //~ manual_highest_one
    };
    // Whereas, this should be linted.
    let _ = if u == 0 {
        //~^ manual_highest_one
        black_box(u);
        todo!()
    } else {
        u.bit_width() - 1
    };

    // --- In match arm ---
    let _ = match u {
        //~^ manual_highest_one
        0 => {
            black_box(u);
            todo!()
        },
        _ => u.bit_width() - 1,
    };
    let _ = match u {
        0 => todo!(),
        _ => {
            black_box(u);
            u.bit_width() - 1 //~ manual_highest_one
        },
    };
    let _ = match u {
        0 => u.bit_width() - 1, //~ manual_highest_one
        _ => todo!(),
    };

    // --- Macro ---
    // `identity!` macro cannot be detected because span is not updated during expansion,
    // so this is a false positive.
    let _ = identity!(31) - u.leading_zeros(); //~ manual_highest_one
    let _ = 31 - identity!(u).leading_zeros(); //~ manual_highest_one
    // We are very conservative about macro, so these should not be linted.
    let _ = 31 - double!(u).leading_zeros();
    let _ = thirty_one!() - u.leading_zeros();
    let _ = (u32::BITS - one!()) - u.leading_zeros();
    let _ = (u32::BITS - u.leading_zeros()) - one!();
    let _ = u32::BITS - (one!() + u.leading_zeros());
    let _ = u32::BITS - (u.leading_zeros() + one!());
    // Whereas, this case should be linted.
    let _ = double!(u).bit_width() - 1; //~ manual_highest_one

    macro_rules! ignore_me {
        () => {
            let _ = 31 - u.leading_zeros();
            let _ = u32::BITS - 1 - u.leading_zeros();
            let _ = u32::BITS - (1 + u.leading_zeros());
        };
    }
    ignore_me!();

    // False negative
    let _ = if u == 0 {
        let v = u;
        31 - v.leading_zeros() //~ manual_highest_one
    } else {
        let _ = "Rust is fast, memory-safe and productive.";
        3
    };
}
