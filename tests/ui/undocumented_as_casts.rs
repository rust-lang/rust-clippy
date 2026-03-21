//@aux-build:proc_macros.rs

#![warn(clippy::undocumented_as_casts)]

extern crate proc_macros;
use proc_macros::{external, with_span};

fn ignore_external() {
    external!(0u32 as u64); // Should not lint
}

fn ignore_proc_macro() {
    with_span!(
        span

        fn converting() {
            let x = 0u32 as u64; // Should not lint
        }
    );
}

fn declared_macro() {
    macro_rules! cast {
        ($x:expr, $t:ty) => {
            // CAST: reason for the cast
            $x as $t
        };
    }

    cast!(0u32, u64);

    // CAST: reason for the cast
    cast!(0u32 as u64, u32);

    macro_rules! cast_no_comment {
        ($x:expr, $t:ty) => {
            $x as $t
            //~^ undocumented_as_casts
        };
    }

    cast_no_comment!(0u32, u64);

    macro_rules! cast_with_comment_after {
        ($x:expr, $t:ty) => {
            $x as $t // CAST: reason for the cast
            //
            //~^^ undocumented_as_casts
        };
    }

    cast_with_comment_after!(0u32, u64);

    macro_rules! add_one {
        ($x:expr) => {
            $x + 1
        };
    }

    // CAST: reason for the cast
    add_one!(0u32 as u64);
}

// Valid Comments

fn line_comment() {
    // CAST: reason
    let _ = 0u32 as u64;
}

fn line_comment_lowercase() {
    // cast: reason
    let _ = 0u32 as u64;
}

fn line_comment_mixed_case() {
    // CaSt: reason
    let _ = 0u32 as u64;
}

fn line_comment_newlines() {
    // CAST: reason

    let _ = 0u32 as u64;
}

fn line_comment_empty() {
    // CAST: reason
    //
    //
    //
    let _ = 0u32 as u64;
}

fn line_comment_with_extras() {
    // This is a description
    // CAST: reason
    let _ = 0u32 as u64;
}

fn line_comment_multiple_casts_same_line() {
    // CAST: reason for both casts
    let _ = 0u32 as u64 + 1u16 as u64;
}

fn line_comment_multiple_casts() {
    // CAST: reason for first cast
    let x = 0u32 as u64;
    // CAST: reason for second cast
    let y = 0u8 as u64;
}

fn line_comment_function_return() -> u64 {
    // CAST: reason
    0u32 as u64
}

fn line_comment_match_block_multiple_arms() {
    let x = 0u32;
    match x {
        0 => {
            // CAST: reason for first cast
            let _ = x as u64;
        },
        _ => {
            // CAST: reason for second cast
            let _ = x as u64;
        },
    }
}

fn line_comment_let_match_block_multiple_arms() {
    let x = 0u32;
    let y = match x {
        0 => {
            // CAST: reason for first cast
            x as u64
        },
        _ => {
            // CAST: reason for second cast
            x as u64
        },
    };
}

fn newline_between_cast_line_comment_and_line_comment() {
    // CAST: reason

    // This is a description
    let _ = 0u32 as u64;
}

fn line_comment_let_match_then_cast() {
    let x = 0u32;
    // CAST: reason for match block cast
    let y = match x {
        0 => {
            // CAST: reason for first cast
            x as u64
        },
        _ => {
            // CAST: reason for second cast
            x as u64
        },
    } as usize;
}

fn block_comment() {
    /* CAST: reason */
    let _ = 0u32 as u64;
}

fn block_comment_newlines() {
    /* CAST: reason */

    let _ = 0u32 as u64;
}

fn block_comment_multiple_line() {
    /* This is a description
     * CAST: reason
     */
    let _ = 0u32 as u64;
}

fn block_comment_multiple_casts_same_line() {
    /* CAST: reason for both casts */
    let _ = 0u32 as u64 + 1u16 as u64;
}

fn block_comment_multiple_casts() {
    /* CAST: reason for first cast */
    let x = 0u32 as u64;
    /* CAST: reason for second cast */
    let y = 0u8 as u64;
}

fn block_comment_function_return() -> u64 {
    /* CAST: reason */
    0u32 as u64
}

fn block_comment_let_match_then_cast() {
    let x = 0u32;
    /* CAST: reason for match block cast */
    let y = match x {
        0 => {
            /* CAST: reason for first cast */
            x as u64
        },
        _ => {
            /* CAST: reason for second cast */
            x as u64
        },
    } as usize;
}

// Invalid Comment

fn no_comment() {
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn trailing_cast_comment() {
    let _ = 0u32 as u64; // CAST: reason
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

fn non_cast_block_comment() {
    /* This is a description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment_newlines() {
    /* This is a description */

    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment_with_extras() {
    /* This is a description
     * This is more description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn no_comment_first_cast() {
    let x = 0u32 as u64;
    //~^ undocumented_as_casts

    // CAST: reason
    let y = 1u32 as u64;
}

fn no_comment_following_cast() {
    // CAST: reason
    let x = 0u32 as u64;

    let y = 1u32 as u64;
    //~^ undocumented_as_casts
}

fn line_cast_comment_before_block_comment() {
    // CAST: reason
    /* This is a description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn line_comment_let_match_then_cast_invalid() {
    let x = 0u32;

    let y = match x {
        //~^ undocumented_as_casts
        0 => {
            // CAST: reason for first cast
            x as u64
        },
        _ => {
            // CAST: reason for second cast
            x as u64
        },
        // CAST: reason for match block cast
    } as usize;
}

fn block_cast_comment_before_line_comment() {
    /* CAST: reason */
    // This is a description
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}

fn block_cast_comment_before_block_comment() {
    /* CAST: reason */
    /* This is a description */
    let _ = 0u32 as u64;
    //~^ undocumented_as_casts
}
