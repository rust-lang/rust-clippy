#![feature(let_chains)]
#![allow(clippy::eq_op, clippy::nonminimal_bool)]

#[rustfmt::skip]
#[warn(clippy::collapsible_if)]
fn main() {
    if true {
        if true {
            println!("No comment, linted");
        }
    }
    //~^^^^^ collapsible_if

    if true {
        // Do not collapse because of this comment
        if true {
            println!("Hello world!");
        }
    }

    if let Some(a) = Some(3) {
        if let Some(b) = Some(4) {
            let _ = a + b;
        }
    }
    //~^^^^^ collapsible_if

    if let Some(a) = Some(3) {
        if a + 1 == 4 {
            let _ = a;
        }
    }
    //~^^^^^ collapsible_if

    if Some(3) == Some(4).map(|x| x - 1) {
        if let Some(b) = Some(4) {
            let _ = b;
        }
    }
    //~^^^^^ collapsible_if
}
