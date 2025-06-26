#![warn(clippy::foo_bar)]

fn main() {
    // test code goes here
    compile_error!("hi");
    //~^ error: hi
}
