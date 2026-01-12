#![warn(clippy::wildcard_enum_match_arm)]

enum Color {
    Red,
    Blue,
    Green,
}

fn main() {
    let c = Color::Red;
    match c {
        Color::Red => println!("Red"),
        _ => unreachable!(),
    }
}
