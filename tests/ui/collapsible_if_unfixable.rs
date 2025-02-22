//@no-rustfix

#[rustfmt::skip]
#[warn(clippy::collapsible_if)]
fn main() {
    let x = "foo";
    let y = "bar";
    if x == "hello" {
        // Not fixable because of the comment, but warned about
        if y == "world" {
            println!("Hello world!");
        }
    }
    //~^^^^^^ collapsible_if

    // The following tests check for the fix of https://github.com/rust-lang/rust-clippy/issues/798
    if x == "hello" {// Not fixable
        if y == "world" {
            println!("Hello world!");
        }
    }
    //~^^^^^ collapsible_if

    if x == "hello" { // Not fixable
        if y == "world" {
            println!("Hello world!");
        }
    }
    //~^^^^^ collapsible_if

    if x == "hello" {
        /* Not fixable */
        if y == "world" {
            println!("Hello world!");
        }
    }
    //~^^^^^^ collapsible_if

    if x == "hello" { /* Not fixable */
        if y == "world" {
            println!("Hello world!");
        }
    }
    //~^^^^^ collapsible_if
}
