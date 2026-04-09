#![warn(clippy::match_same_arms)]

fn main() {
    let x = 2;

    match x {
        1 => println!("same"),
        2 => println!("different"),
        3 => println!("same"), //~ ERROR: these match arms have identical bodies
        4 => println!("same"),
        _ => println!("other"),
    }
}