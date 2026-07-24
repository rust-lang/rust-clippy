#![warn(clippy::disallowed_pub_api_types)]
#![allow(clippy::result_unit_err, clippy::unused_self)]

pub fn bad_pub_fn() -> Result<(), ()> {
    //~^ disallowed_pub_api_types
    Ok(())
}

pub fn bad_pub_fn_arg(_arg: Result<(), ()>) {
    //~^ disallowed_pub_api_types
}

fn private_fn() -> Result<(), ()> {
    // should not trigger
    Ok(())
}

fn private_fn_arg(_arg: Result<(), ()>) {
    // should not trigger
}

pub struct Struct;

impl Struct {
    pub fn bad_method(&self) -> Option<i32> {
        //~^ disallowed_pub_api_types
        Some(42)
    }

    pub fn bad_method_arg(&self, _arg: Option<i32>) {
        //~^ disallowed_pub_api_types
    }

    fn private_method(&self) -> Option<i32> {
        Some(42)
    }

    fn private_method_arg(&self, _arg: Option<i32>) {}
}

pub trait Trait {
    fn bad_trait_method() -> Result<(), ()>;
    //~^ disallowed_pub_api_types

    fn bad_trait_method_arg(_arg: Result<(), ()>);
    //~^ disallowed_pub_api_types
}

pub enum NestedStruct {
    One(Result<(), ()>),
    Two(Option<i32>),
    Three(String),
}

pub fn get_nested_struct() -> NestedStruct {
    // Should not trigger. Wrapping the type is fine
    NestedStruct::One(Ok(()))
}

pub enum InputOnly {
    Pretty,
    Raw,
}

pub fn get_input_type() -> crate::InputOnly {
    //~^ disallowed_pub_api_types
    InputOnly::Pretty
}

pub fn bad_pub_fn_input_only_arg(_arg: crate::InputOnly) {
    //~^ disallowed_pub_api_types
}

fn main() {}
