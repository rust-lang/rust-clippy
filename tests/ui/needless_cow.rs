#![allow(
    clippy::borrow_deref_ref,
    clippy::collapsible_else_if,
    clippy::deref_addrof,
    clippy::if_same_then_else
)]
#![warn(clippy::needless_cow)]

use std::borrow::Cow;

// FIXME: Should not lint.
fn passed_to_fn() {
    fn helper(_: Cow<'static, str>) {}

    let x = if true { Cow::Borrowed("") } else { Cow::Borrowed("") };
    let y = x;

    helper(y);
}

// FIXME: Should not lint.
fn passed_to_fn_mut_ref() {
    fn helper(_: &mut Cow<'static, str>) {}

    let x = if true { Cow::Borrowed("") } else { Cow::Borrowed("") };
    let mut y = x;

    helper(&mut y);
}

// FIXME: Should not lint.
fn returning() -> Cow<'static, str> {
    if true { Cow::Borrowed("") } else { Cow::Borrowed("") }
}

fn main() {
    if true {
        Cow::Owned(String::new())
    } else {
        if true { Cow::Borrowed("") } else { Cow::Borrowed(&*"") }
    };

    if true {
        Cow::Borrowed(" ")
    } else {
        Cow::Borrowed("")
    };

    match 1 {
        1 => Cow::Owned(String::new()),
        2 => Cow::Borrowed(""),
        3 => Cow::Borrowed(""),
        4 => Cow::Owned("aa".to_string()),
        _ => return,
    };

    if true {
        "".into()
    } else {
        Cow::Owned(String::new())
    };

    if true {
        Cow::from("")
    } else {
        Cow::Owned(String::new())
    };

    // Probably a bit too pedantic to lint this.
    _ = Cow::Borrowed("");
}
