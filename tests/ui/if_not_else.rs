#![warn(clippy::all)]
#![warn(clippy::if_not_else)]

fn bla() -> bool {
    unimplemented!()
}

fn main() {
    if !bla() {
        println!("Bugs");
    } else {
        println!("Bunny");
    }
    if 4 != 5 {
        println!("Bugs");
    } else {
        println!("Bunny");
    }

    // These won't get flagged because the body of the `else` block is
    // longer than the body of the then-block.
    if !bla() {
        println!("Bugs");
    } else {
        println!("Bunny");
        println!("Daffy");
        println!("Duck");
    }
    if 4 != 5 {
        println!("Bugs");
    } else {
        println!("Bunny");
        println!("Daffy");
        println!("Duck");
    }
}
