//@ no-rustfix
// https://github.com/rust-lang/rust-clippy/issues/15353
#![warn(clippy::missing_panics_doc, clippy::missing_safety_doc, clippy::missing_errors_doc)]

pub struct Error;

/// <div>
/// # Panics # not header
///
/// Here's where some panic docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure this thing isn't mistakened for a header,
/// since that won't work.
///
/// </div>
pub fn panicking1() {
    //~^ missing_panics_doc
    panic!();
}

/// <div>
/// Panics
/// ===----
///
/// Here's where some panic docs are supposed to appear,
/// but don't, because of the div.
///
/// Make sure this thing isn't mistakened for a header,
/// since that won't work.
///
/// </div>
pub fn panicking2() {
    //~^ missing_panics_doc
    panic!();
}
