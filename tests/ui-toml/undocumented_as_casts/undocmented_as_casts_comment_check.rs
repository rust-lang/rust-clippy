//@aux-build:../../ui/auxiliary/proc_macros.rs
//@revisions: default
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/undocumented_as_casts/default

#![warn(clippy::undocumented_as_casts)]

extern crate proc_macros;
use proc_macros::{external, with_span};

fn ignore_external() {
    external!(&0i32 as *const i32);
}

fn ignore_proc_macro() {
    with_span!(
        span

        fn converting() {
            let p: *const u32 = &42_u32;
            let _ = p as *mut i32;
        }
    );
}

fn declared_macro() {
    let p: *const u32 = &42_u32;

    macro_rules! mut_cast_no_comment {
        ($x:expr, $t:ty) => {
            $x as $t
            //~^ undocumented_as_casts
        };
    }

    mut_cast_no_comment!(p, *mut i32);

    macro_rules! const_cast_no_comment {
        ($x:expr, $t:ty) => {
            $x as $t
            //~^ undocumented_as_casts
        };
    }

    const_cast_no_comment!(p, *const i32);
}

// Valid Comments

fn line_comment() {
    let p: *const u32 = &42_u32;
    // CAST: reason
    let _ = p as *const i32;
}

fn line_comment_lowercase() {
    let p: *const u32 = &42_u32;
    // cast: reason
    let _ = p as *const i32;
}

fn line_comment_mixed_case() {
    let p: *const u32 = &42_u32;
    // CaSt: reason
    let _ = p as *const i32;
}

fn line_comment_newlines() {
    let p: *const u32 = &42_u32;
    // CAST: reason

    let _ = p as *const i32;
}

fn line_comment_empty() {
    let p: *const u32 = &42_u32;
    // CAST: reason
    //
    //
    //
    let _ = p as *const i32;
}

fn line_comment_with_extras() {
    let p: *const u32 = &42_u32;
    // This is a description
    // CAST: reason
    let _ = p as *const i32;
}

fn line_comment_multiple_casts_same_line() {
    let p: *const u32 = &42_u32;
    // CAST: reason for both casts
    let _ = (p as *const i32, p as *mut i32);
}

fn line_comment_multiple_casts() {
    let p: *const u32 = &42_u32;
    // CAST: reason for first cast
    let x = p as *const i32;
    // CAST: reason for second cast
    let y = p as *mut i32;
}

fn line_comment_function_return() -> *const i32 {
    let p: *const u32 = &42_u32;
    // CAST: reason
    p as *const i32
}

fn line_comment_match_block_multiple_arms() {
    let p: *const u32 = &42_u32;
    let cond = true;
    match cond {
        true => {
            // CAST: reason for first cast
            let _ = p as *const i32;
        },
        _ => {
            // CAST: reason for second cast
            let _ = p as *mut i32;
        },
    }
}

fn line_comment_let_match_block_multiple_arms() {
    let p: *const u32 = &42_u32;
    let cond = true;
    let y = match cond {
        true => {
            // CAST: reason for first cast
            p as *const i32
        },
        _ => {
            // CAST: reason for second cast
            p as *const i32
        },
    };
}

fn newline_between_cast_line_comment_and_line_comment() {
    let p: *const u32 = &42_u32;
    // CAST: reason

    // This is a description
    let _ = p as *const i32;
}

fn line_comment_let_match_then_cast() {
    let p: *const u32 = &42_u32;
    let cond = true;
    // CAST: reason for match block cast
    let y = match cond {
        true => {
            // CAST: reason for first cast
            p as *const i32
        },
        _ => {
            // CAST: reason for second cast
            p as *const i32
        },
    } as *mut i32;
}

fn block_comment() {
    let p: *const u32 = &42_u32;
    /* CAST: reason */
    let _ = p as *const i32;
}

fn block_comment_newlines() {
    let p: *const u32 = &42_u32;
    /* CAST: reason */

    let _ = p as *const i32;
}

fn block_comment_multiple_line() {
    let p: *const u32 = &42_u32;
    /* This is a description
     * CAST: reason
     */
    let _ = p as *const i32;
}

fn block_comment_multiple_casts_same_line() {
    let p: *const u32 = &42_u32;
    /* CAST: reason for both casts */
    let _ = (p as *const i32, p as *mut i32);
}

fn block_comment_multiple_casts() {
    let p: *const u32 = &42_u32;
    /* CAST: reason for first cast */
    let x = p as *const i32;
    /* CAST: reason for second cast */
    let y = p as *mut i32;
}

fn block_comment_function_return() -> *const i32 {
    let p: *const u32 = &42_u32;
    /* CAST: reason */
    p as *const i32
}

fn block_comment_let_match_then_cast() {
    let p: *const u32 = &42_u32;
    let cond = true;
    /* CAST: reason for match block cast */
    let y = match cond {
        true => {
            /* CAST: reason for first cast */
            p as *const i32
        },
        _ => {
            /* CAST: reason for second cast */
            p as *const i32
        },
    } as *mut i32;
}

// Invalid Comment

fn no_comment() {
    let p: *const u32 = &42_u32;
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn trailing_cast_comment() {
    let p: *const u32 = &42_u32;
    let _ = p as *const i32; // CAST: reason
    //
    //~^^ undocumented_as_casts
}

fn non_cast_comment() {
    let p: *const u32 = &42_u32;
    // This is a description
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn non_cast_comment_newlines() {
    let p: *const u32 = &42_u32;
    // This is a description

    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn non_cast_comment_with_extras() {
    let p: *const u32 = &42_u32;
    // This is a description
    // This is more description
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment() {
    let p: *const u32 = &42_u32;
    /* This is a description */
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment_newlines() {
    let p: *const u32 = &42_u32;
    /* This is a description */

    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn non_cast_block_comment_with_extras() {
    let p: *const u32 = &42_u32;
    /* This is a description
     * This is more description */
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn no_comment_first_cast() {
    let p: *const u32 = &42_u32;
    let x = p as *const i32;
    //~^ undocumented_as_casts

    // CAST: reason
    let y = p as *mut i32;
}

fn no_comment_following_cast() {
    let p: *const u32 = &42_u32;
    // CAST: reason
    let x = p as *const i32;

    let y = p as *mut i32;
    //~^ undocumented_as_casts
}

fn line_cast_comment_before_block_comment() {
    let p: *const u32 = &42_u32;
    // CAST: reason
    /* This is a description */
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn line_comment_let_match_then_cast_invalid() {
    let p: *const u32 = &42_u32;
    let cond = true;

    let y = match cond {
        //~^ undocumented_as_casts
        true => {
            // CAST: reason for first cast
            p as *const i32
        },
        _ => {
            // CAST: reason for second cast
            p as *const i32
        },
        // CAST: reason for match block cast
    } as *mut i32;
}

fn block_cast_comment_before_line_comment() {
    let p: *const u32 = &42_u32;
    /* CAST: reason */
    // This is a description
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn block_cast_comment_before_block_comment() {
    let p: *const u32 = &42_u32;
    /* CAST: reason */
    /* This is a description */
    let _ = p as *const i32;
    //~^ undocumented_as_casts
}

fn main() {}
