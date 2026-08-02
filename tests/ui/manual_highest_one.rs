#![warn(clippy::manual_highest_one)]

use std::num::NonZeroU32;

fn do_something<T>(x: T) {
    todo!()
}

fn may_change_state() {
    todo!()
}

// This macro will be allowed by `clippy_utils::consts::integer_const`
macro_rules! identity {
    ( $e:expr ) => {
        $e
    };
}

// These macro will be denied by `clippy_utils::consts::integer_const`
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
    let nz = NonZeroU32::new(5).unwrap();

    // --- Integer type ---
    let _ = 31 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - 1 - u.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - u.leading_zeros() - 1; //~ manual_highest_one
    let _ = u32::BITS - (1 + u.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS - (u.leading_zeros() + 1); //~ manual_highest_one
    let _ = u.bit_width() - 1; //~ manual_highest_one
    // False negative
    let _ = u32::BITS.checked_sub(u.leading_zeros() + 1);

    // --- Nonzero type ---
    let _ = 31 - nz.leading_zeros(); //~ manual_highest_one
    let _ = u32::BITS - 1 - nz.leading_zeros(); //~ manual_highest_one
    {
        #![expect(clippy::mismatched_bit_width_type)]
        // this trigger `mismatched_bit_width_type`
        let _ = u32::BITS - nz.leading_zeros();
        let _ = u32::BITS - nz.leading_zeros() - 1; //~ manual_highest_one
    }
    let _ = u32::BITS - (1 + nz.leading_zeros()); //~ manual_highest_one
    let _ = u32::BITS - (nz.leading_zeros() + 1); //~ manual_highest_one
    // False negative
    let _ = nz.bit_width().get() - 1;
    // False positive: Currently, `integer_const` does not support `NonZero`
    // and related aliases. It is better to enhance `integer_const`.
    let _ = NonZeroU32::BITS - 1 - nz.leading_zeros();

    // --- In if block ---
    {
        #![expect(clippy::unnecessary_lazy_evaluations)]
        let _ = u.highest_one().unwrap_or_else(|| 0);
        let _ = if u == 0 {
            //~^ manual_highest_one
            0
        } else {
            31 - u.leading_zeros()
        };
    }
    let _ = if 0 == u {
        //~^ manual_highest_one
        todo!()
    } else {
        31 - u.leading_zeros()
    };
    let _ = if u != 0 {
        //~^ manual_highest_one
        31 - u.leading_zeros()
    } else {
        do_something(u);
        1
    };
    let _ = if 0 != u {
        //~^ manual_highest_one
        31 - u.leading_zeros()
    } else {
        do_something(u);
        1
    };
    let _ = if u > 0 {
        //~^ manual_highest_one
        31 - u.leading_zeros()
    } else {
        do_something(u);
        1
    };
    // This is not linted
    let _ = if u == 0 {
        todo!()
    } else {
        may_change_state();
        31 - u.leading_zeros() //~ manual_highest_one
    };
    // False negative
    #[expect(clippy::blocks_in_conditions)]
    let _ = if {
        do_something(u);
        u == 0
    } {
        todo!()
    } else {
        31 - u.leading_zeros() //~ manual_highest_one
    };

    // --- In match arm ---
    let _ = match u {
        //~^ manual_highest_one
        0 => {
            do_something(u);
            todo!()
        },
        _ => 31 - u.leading_zeros(),
    };
    // This is not linted
    let _ = match u {
        0 => todo!(),
        _ => {
            may_change_state();
            31 - u.leading_zeros() //~ manual_highest_one
        },
    };

    // --- Macro ---
    let _ = identity!(31) - u.leading_zeros(); //~ manual_highest_one
    let _ = 31 - identity!(u).leading_zeros(); //~ manual_highest_one
    let _ = 31 - double!(u).leading_zeros(); //~ manual_highest_one
    let _ = thirty_one!() - u.leading_zeros();
    let _ = (u32::BITS - one!()) - u.leading_zeros();
    let _ = (u32::BITS - u.leading_zeros()) - one!();
    let _ = u32::BITS - (one!() + u.leading_zeros());
    let _ = u32::BITS - (u.leading_zeros() + one!());

    macro_rules! ignore_me {
        () => {
            let _ = 31 - u.leading_zeros();
            let _ = u32::BITS - 1 - u.leading_zeros();
            let _ = u32::BITS - (1 + u.leading_zeros());
        };
    }
    ignore_me!();

    // False negatives
    let _ = if u == 0 {
        let v = u;
        31 - v.leading_zeros() //~ manual_highest_one
    } else {
        let _ = "Rust is fast, memory-safe and productive.";
        3
    };
    let _ = u32::BITS.checked_sub(u.leading_zeros() + 1);
}
