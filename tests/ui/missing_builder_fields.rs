#![warn(clippy::missing_builder_fields)]

use derive_builder::Builder;

#[derive(Builder)]
struct Foo {
    required_a: u32,
    required_b: u32,
    #[builder(default)]
    optional: u32,
}

fn main() {
    // Bad: both required fields missing
    let _ = FooBuilder::default().build();
    //~^ ERROR: builder is missing required fields: required_a, required_b

    // Bad: `required_b` missing
    let _ = FooBuilder::default().required_a(1).build();
    //~^ ERROR: builder is missing required field: required_b

    // Good: both required fields set, optional omitted
    let _ = FooBuilder::default().required_a(1).required_b(2).build();

    // Good: all fields set
    let _ = FooBuilder::default().required_a(1).required_b(2).optional(99).build();

    // Good: mutable builder — we can't track setters through a variable, so we don't lint
    let mut builder = FooBuilder::default();
    builder.required_a(1);
    builder.required_b(2);
    let _ = builder.build();
}
