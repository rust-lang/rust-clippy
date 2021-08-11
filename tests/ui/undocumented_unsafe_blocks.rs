#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(unused_unsafe)]

fn good_basic() {
    // SAFETY
    unsafe {}
}

fn good_newlines() {
    // SAFETY
    //
    //
    //
    unsafe {}
}

#[rustfmt::skip]
fn good_whitespaces() {
    // SAFETY



    unsafe {}
}

fn good_block() {
    /* SAFETY */
    unsafe {}
}

#[rustfmt::skip]
fn good_inline_block() {
    /* SAFETY */ unsafe {}
}

fn good_statement() {
    let _ =
        // SAFETY
        unsafe {};
}

fn bad_no_comment() {
    unsafe {}
}

fn bad_comment_after_block() {
    unsafe {} // SAFETY
}

fn bad_comment_after_block2() {
    unsafe {
        //
    } // SAFETY
    // SAFETY
}

fn bad_comment_inside_block() {
    unsafe {
        // SAFETY
    }
}

#[rustfmt::skip]
fn bad_two_blocks_one_comment() {
    // SAFETY
    unsafe {}; unsafe {}
}

#[rustfmt::skip]
fn bad_block_comment_inside_block() {
    unsafe { /* SAFETY */ }
}

fn main() {}
