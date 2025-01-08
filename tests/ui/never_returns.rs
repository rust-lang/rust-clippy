#![warn(clippy::never_returns)]
#![allow(clippy::empty_loop)]

fn stuff() {}

fn never_returns() {
    //~^ error: function never returns, but is typed to return
    loop {
        stuff()
    }
}

fn never_returns_with_ty() -> u8 {
    //~^ error: function never returns, but is typed to return a `u8`
    loop {
        stuff()
    }
}

fn never_returns_conditionally(cond: bool) -> u8 {
    //~^ error: function never returns, but is typed to return a `u8`
    if cond { std::process::exit(0) } else { panic!() }
}

fn returns_unit_implicit(cond: bool) {
    if cond {}
}

fn returns_in_loop(cond: bool) -> u8 {
    loop {
        if cond {
            break 1;
        }
    }
}

trait ExampleTrait {
    fn example(self) -> u8;
}

// Should not lint, as the return type is required by the trait
impl ExampleTrait for () {
    fn example(self) -> u8 {
        loop {}
    }
}

fn main() {}
