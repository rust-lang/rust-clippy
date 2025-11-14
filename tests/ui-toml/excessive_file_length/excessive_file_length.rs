#![warn(clippy::excessive_file_length)]
//~^ ERROR: this file has too many lines

// This file should trigger the lint because it has more than 10 lines
// (configured in clippy.toml)

fn main() {
    println!("Line 1");
    println!("Line 2");
    println!("Line 3");
    println!("Line 4");
    println!("Line 5");
    println!("Line 6");
    println!("Line 7");
    // This file now has more than 10 lines and should trigger the lint
}
