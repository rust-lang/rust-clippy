//@compile-flags: --test

#![warn(clippy::module_name_repetitions)]
#![allow(dead_code)]

mod foo {
    pub fn foo() {}
    pub fn foo_bar() {}
    //~^ ERROR: item name starts with its containing module's name
    //~| NOTE: `-D clippy::module-name-repetitions` implied by `-D warnings`
    pub fn bar_foo() {}
    //~^ ERROR: item name ends with its containing module's name
    pub struct FooCake;
    //~^ ERROR: item name starts with its containing module's name
    pub enum CakeFoo {}
    //~^ ERROR: item name ends with its containing module's name
    pub struct Foo7Bar;
    //~^ ERROR: item name starts with its containing module's name

    // Should not warn
    pub struct Foobar;

    // #12544 - shouldn't warn if the only part aside from the the module name is a preposition
    pub fn to_foo() {}
    pub fn into_foo() {}
    pub fn as_foo() {}
    pub fn from_foo() {}
    pub trait IntoFoo {}
    pub trait ToFoo {}
    pub trait AsFoo {}
    pub trait FromFoo {}
}

fn main() {}
