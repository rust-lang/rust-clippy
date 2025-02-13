#![warn(clippy::parse_to_string)]

fn main() {
    let bad: u64 = 42_u32.to_string().parse().unwrap();
    //~^ ERROR: parsing
}
