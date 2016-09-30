#![feature(plugin)]
#![plugin(clippy)]

#![deny(ok_if_let)]

fn str_to_int(x: &str) -> i32 {
    if let Some(y) = x.parse().ok() { 
    //~^ERROR Matching on `Some` with `ok()` is redundant
        y
    } else {
        0
    }
}
fn main() {
    let y = str_to_int("1");
    println!("{}", y);
}
