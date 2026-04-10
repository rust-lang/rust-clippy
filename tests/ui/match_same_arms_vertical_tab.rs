#![warn(clippy::match_same_arms)]
#![allow(clippy::manual_range_patterns)]

fn main() {
    let x = 1;

    // Basic vertical tab case
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {}
    }

    // Multiple duplicate arms
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        3 => println!("same"),
        _ => {}
    }
}