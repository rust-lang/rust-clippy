#![warn(clippy::match_on_vec_items)]
#![allow(clippy::useless_vec)]
//@no-rustfix

#[clippy::msrv = "1.52.0"]
fn or_patterns() {
    let arr = vec![0, 1, 2, 3];
    match arr[1] {
        //~^ ERROR: indexing into a vector may panic
        0 | 1 => println!("0 or 1"),
        _ => println!("Hello, World!"),
    }
}
