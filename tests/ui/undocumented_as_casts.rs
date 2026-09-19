//@aux-build:proc_macros.rs
//@no-rustfix

#![warn(clippy::undocumented_as_casts)]

extern crate proc_macros;
use proc_macros::{external, with_span};

// Macros: `// CAST:` must precede the call site

fn external_macro() {
    external!(0u32 as u64);
    //~^ undocumented_as_casts

    // CAST: explanation for the cast
    external!(0u32 as u64);
}

fn proc_macro() {
    with_span!(
        span1
        //~^ undocumented_as_casts

        fn converting1() {
            let x = 0u32 as u64;
        }
    );

    with_span!(
        // CAST: explanation for the cast
        span2

        fn converting2() {
            let x = 0u32 as u64;
        }
    );
}

fn declared_macro() {
    macro_rules! cast {
        ($x:expr, $t:ty) => {
            $x as $t
        };
    }

    cast!(0u32, u64);
    //~^ undocumented_as_casts

    // CAST: explanation for the cast
    cast!(0u32, u64);

    // CAST: explanation for the cast
    cast!(0u32 as u64, u32);

    macro_rules! cast_with_comment_in_macro_body {
        ($x:expr, $t:ty) => {
            // CAST: explanation in the macro body does not count
            $x as $t
        };
    }

    cast_with_comment_in_macro_body!(0u32, u64);
    //~^ undocumented_as_casts

    macro_rules! add_one {
        ($x:expr) => {
            $x + 1
        };
    }

    // CAST: explanation for the cast
    add_one!(0u32 as u64);
}

// Valid: contiguous preceding `// CAST:` comments

fn line_comment() {
    // CAST: explanation
    let _ = 0u32 as u64;
}

fn line_comment_newlines() {
    // CAST: explanation

    let _ = 0u32 as u64;
}

fn line_comment_empty() {
    // CAST: explanation
    //
    //
    //
    let _ = 0u32 as u64;
}

fn line_comment_with_extras() {
    // This is a description
    // CAST: explanation
    let _ = 0u32 as u64;
}

fn line_comment_multiple_casts_same_line() {
    // CAST: explanation for both casts
    let _ = 0u32 as u64 + 1u16 as u64;
}

fn line_comment_multiple_casts() {
    // CAST: explanation for first cast
    let x = 0u32 as u64;
    // CAST: explanation for second cast
    let y = 0u8 as u64;
}

fn line_comment_function_return() -> u64 {
    // CAST: explanation
    0u32 as u64
}

fn line_comment_match_block_multiple_arms() {
    let x = 0u32;
    match x {
        0 => {
            // CAST: explanation for first cast
            let _ = x as u64;
        },
        _ => {
            // CAST: explanation for second cast
            let _ = x as u64;
        },
    }
}

fn line_comment_let_match_block_multiple_arms() {
    let x = 0u32;
    let y = match x {
        0 => {
            // CAST: explanation for first cast
            x as u64
        },
        _ => {
            // CAST: explanation for second cast
            x as u64
        },
    };
}

fn newline_between_cast_line_comment_and_line_comment() {
    // CAST: explanation

    // This is a description
    let _ = 0u32 as u64;
}

fn line_comment_let_match_then_cast() {
    let x = 0u32;
    // CAST: explanation for match block cast
    let y = match x {
        0 => {
            // CAST: explanation for first cast
            x as u64
        },
        _ => {
            // CAST: explanation for second cast
            x as u64
        },
    } as usize;
}

// Invalid: missing, wrong case, trailing, non-`//`, or wrong position

fn no_comment() {
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn line_comment_lowercase() {
    // cast: reason
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn line_comment_mixed_case() {
    // CaSt: reason
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn trailing_cast_comment() {
    let _ = 0u32 as u64; // CAST: explanation
    //
    //~^^ undocumented_as_casts
}

fn non_cast_comment() {
    // This is a description
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn non_cast_comment_newlines() {
    // This is a description

    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn non_cast_comment_with_extras() {
    // This is a description
    // This is more description
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn block_comment() {
    /* CAST: explanation */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment() {
    /* This is a description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn line_cast_comment_before_block_comment() {
    // CAST: explanation
    /* This is a description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn no_comment_first_cast() {
    let x = 0u32 as u64;
    //~^ undocumented_as_casts

    // CAST: explanation
    let y = 1u32 as u64;
}

fn no_comment_following_cast() {
    // CAST: explanation
    let x = 0u32 as u64;

    let y = 1u32 as u64;
    //~^ undocumented_as_casts
}

fn line_comment_let_match_then_cast_invalid() {
    let x = 0u32;

    let y = match x {
        //~^ undocumented_as_casts
        0 => {
            // CAST: explanation for first cast
            x as u64
        },
        _ => {
            // CAST: explanation for second cast
            x as u64
        },
        // CAST: explanation for match block cast
    } as usize;
}
