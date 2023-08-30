#![allow(clippy::match_single_binding)]

fn main() {
    println!();
    println!("");
    //~^ ERROR: empty string literal in `println!`
    //~| NOTE: `-D clippy::println-empty-string` implied by `-D warnings`

    match "a" {
        _ => println!(""),
        //~^ ERROR: empty string literal in `println!`
    }

    eprintln!();
    eprintln!("");
    //~^ ERROR: empty string literal in `eprintln!`

    match "a" {
        _ => eprintln!(""),
        //~^ ERROR: empty string literal in `eprintln!`
    }
}
