//@compile-flags: -C incremental=target/debug/test/incr

// see https://github.com/rust-lang/rust-clippy/issues/10969

fn main() {
    let s = "Hello, world!";
    println!("{}", s.to_string());
    //~^ ERROR: `to_string` applied to a type that implements `Display` in `println!` args
    //~| NOTE: `-D clippy::to-string-in-format-args` implied by `-D warnings`
}
