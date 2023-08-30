#![allow(clippy::assertions_on_constants, clippy::equatable_if_let, clippy::needless_if)]

#[rustfmt::skip]
#[warn(clippy::collapsible_if)]
#[warn(clippy::collapsible_else_if)]

fn main() {
    let x = "hello";
    let y = "world";
    // Collapse `else { if .. }` to `else if ..`
    if x == "hello" {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
    //~| NOTE: `-D clippy::collapsible-else-if` implied by `-D warnings`
        if y == "world" {
            println!("world!")
        }
    }

    if x == "hello" {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if let Some(42) = Some(42) {
            println!("world!")
        }
    }

    if x == "hello" {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if y == "world" {
            println!("world")
        }
        else {
            println!("!")
        }
    }

    if x == "hello" {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if let Some(42) = Some(42) {
            println!("world")
        }
        else {
            println!("!")
        }
    }

    if let Some(42) = Some(42) {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if let Some(42) = Some(42) {
            println!("world")
        }
        else {
            println!("!")
        }
    }

    if let Some(42) = Some(42) {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if x == "hello" {
            println!("world")
        }
        else {
            println!("!")
        }
    }

    if let Some(42) = Some(42) {
        print!("Hello ");
    } else {
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if let Some(42) = Some(42) {
            println!("world")
        }
        else {
            println!("!")
        }
    }

    if x == "hello" {
        print!("Hello ");
    } else {
        #[cfg(not(roflol))]
        if y == "world" {
            println!("world!")
        }
    }
}

#[rustfmt::skip]
#[allow(dead_code)]
fn issue_7318() {
    if true { println!("I've been resolved!")
    }else{
    //~^ ERROR: this `else { if .. }` block can be collapsed
        if false {}
    }
}
