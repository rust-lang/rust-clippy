#![warn(clippy::struct_field_names)]

struct Data {
    a_data: u8,
    b_data: u8,
    c_data: u8,
    d_data: u8,
}
struct Data2 {
    //~^ ERROR: all fields have the same postfix
    a_data: u8,
    b_data: u8,
    c_data: u8,
    d_data: u8,
    e_data: u8,
}
enum Foo {
    AFoo,
    BFoo,
    CFoo,
    DFoo,
}
enum Foo2 {
    //~^ ERROR: all variants have the same postfix
    AFoo,
    BFoo,
    CFoo,
    DFoo,
    EFoo,
}

// This should not trigger the lint as one of it's impl has allow attribute
struct Data3 {
    a_data: u8,
    b_data: u8,
    c_data: u8,
    d_data: u8,
    e_data: u8,
}

#[allow(clippy::struct_field_names)]
impl Data3 {}

// This should not trigger the lint as one of it's impl has allow attribute
enum Foo3 {
    AFoo,
    BFoo,
    CFoo,
    DFoo,
    EFoo,
}

#[allow(clippy::enum_variant_names)]
impl Foo3 {}

fn main() {}
