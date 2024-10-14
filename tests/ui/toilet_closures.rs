#![warn(clippy::toilet_closures)]

fn main() {
    // Should lint
    let toliet: fn(u8) = |_| ();
    //~^ error: defined a 'toilet closure'
    let toliet_with_arg: fn(u8) = |_to_drop| ();
    //~^ error: defined a 'toilet closure'
    let toliet_with_typed_arg: fn(u8) = |_to_drop: u8| ();
    //~^ error: defined a 'toilet closure'
    let toliet_with_braces: fn(u8) = |_| {};
    //~^ error: defined a 'toilet closure'

    // Should not lint
    let toilet_higher_ranked: fn(&u8) = |_| ();
    let closure_multi_arg: fn(u8, u8) = |_, _| ();
    let closure_does_stuff: fn(u8) = |_| println!("Stuff!");
}
