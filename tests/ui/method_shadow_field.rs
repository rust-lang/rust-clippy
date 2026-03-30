#![warn(clippy::method_shadow_field)]

struct Circle {
    radius: f32,
    pub diameter: f32,
    //
    bar: i32,
    pub var: i32,
    //
    extra: i32,
}

impl Circle {
    // Shadows pub or non-pub field names:
    fn radius(self) {}
    //~^ method_shadow_field
    fn diameter(self) {}
    //~^ method_shadow_public_field

    // Do not shadow anything
    fn other(self) {}
    fn another(self) {}

    // Only methods are linted
    fn extra() {}
}

union Counter {
    small: i8,
    pub large: i64,
}

impl Counter {
    // Shadows pub or non-pub field names:
    fn small(self) {}
    //~^ method_shadow_field
    fn large(self) {}
    //~^ method_shadow_public_field

    // Do not shadow anything
    fn tick(self) {}
}

enum Numbers {
    One,
    Two,
    Last,
}

impl Numbers {
    // fields differ in naming from methods: One vs one(), so they don't clash

    // Do not shadow anything
    fn smallest(self) {}
    fn last(self) {}
}

fn main() {}
