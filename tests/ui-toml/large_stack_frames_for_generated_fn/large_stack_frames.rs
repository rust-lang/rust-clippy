//@compile-flags: --test
#![allow(unused)]
#![warn(clippy::large_stack_frames)]

fn main() {
    //~^ large_stack_frames
    println!("Hello, world!");
}

#[cfg(test)]
#[allow(clippy::large_stack_frames)]
mod test {
    #[test]
    fn main_test() {}
}
