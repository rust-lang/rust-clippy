// aux-build:pub_tuple_struct_with_private_fields.rs

#![feature(plugin)]
#![plugin(clippy)]
#![allow(unused)]
#![deny(pub_tuple_struct_with_private_fields, match_private_tuple_struct_fields)]

pub struct Foo(i32); //~ ERROR the number of fields in this tuple struct

// apparently rustc treats this one like a unit struct
pub struct Bar();

pub struct Boo(i32, i32); //~ ERROR the number of fields in this tuple struct

extern crate pub_tuple_struct_with_private_fields;

fn main() {
    use pub_tuple_struct_with_private_fields::*;
    let Foo(_) = foo(); //~ ERROR if the author of `pub_tuple_struct_with_private_fields::Foo` changes the number
}
