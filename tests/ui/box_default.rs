#![warn(clippy::box_default)]
#![allow(clippy::default_constructed_unit_structs)]

#[derive(Default)]
struct ImplementsDefault;

struct OwnDefault;

impl OwnDefault {
    fn default() -> Self {
        Self
    }
}

macro_rules! outer {
    ($e: expr) => {
        $e
    };
}

fn main() {
    let _string: Box<String> = Box::new(Default::default());
    //~^ ERROR: `Box::new(_)` of default value
    //~| NOTE: `-D clippy::box-default` implied by `-D warnings`
    let _byte = Box::new(u8::default());
    //~^ ERROR: `Box::new(_)` of default value
    let _vec = Box::new(Vec::<u8>::new());
    //~^ ERROR: `Box::new(_)` of default value
    let _impl = Box::new(ImplementsDefault::default());
    //~^ ERROR: `Box::new(_)` of default value
    let _impl2 = Box::new(<ImplementsDefault as Default>::default());
    //~^ ERROR: `Box::new(_)` of default value
    let _impl3: Box<ImplementsDefault> = Box::new(Default::default());
    //~^ ERROR: `Box::new(_)` of default value
    let _own = Box::new(OwnDefault::default()); // should not lint
    let _in_macro = outer!(Box::new(String::new()));
    //~^ ERROR: `Box::new(_)` of default value
    let _string_default = outer!(Box::new(String::from("")));
    //~^ ERROR: `Box::new(_)` of default value
    let _vec2: Box<Vec<ImplementsDefault>> = Box::new(vec![]);
    //~^ ERROR: `Box::new(_)` of default value
    let _vec3: Box<Vec<bool>> = Box::new(Vec::from([]));
    //~^ ERROR: `Box::new(_)` of default value
    let _vec4: Box<_> = Box::new(Vec::from([false; 0]));
    //~^ ERROR: `Box::new(_)` of default value
    let _more = ret_ty_fn();
    call_ty_fn(Box::new(u8::default()));
    //~^ ERROR: `Box::new(_)` of default value
    issue_10381();

    // `Box::<Option<_>>::default()` would be valid here, but not `Box::default()` or
    // `Box::<Option<[closure@...]>::default()`
    //
    // Would have a suggestion after https://github.com/rust-lang/rust/blob/fdd030127cc68afec44a8d3f6341525dd34e50ae/compiler/rustc_middle/src/ty/diagnostics.rs#L554-L563
    let mut unnameable = Box::new(Option::default());
    let _ = unnameable.insert(|| {});
}

fn ret_ty_fn() -> Box<bool> {
    Box::new(bool::default())
    //~^ ERROR: `Box::new(_)` of default value
}

#[allow(clippy::boxed_local)]
fn call_ty_fn(_b: Box<u8>) {
    issue_9621_dyn_trait();
}

use std::io::{Read, Result};

impl Read for ImplementsDefault {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

fn issue_9621_dyn_trait() {
    let _: Box<dyn Read> = Box::new(ImplementsDefault::default());
    //~^ ERROR: `Box::new(_)` of default value
    issue_10089();
}

fn issue_10089() {
    let _closure = || {
        #[derive(Default)]
        struct WeirdPathed;

        let _ = Box::new(WeirdPathed::default());
        //~^ ERROR: `Box::new(_)` of default value
    };
}

fn issue_10381() {
    #[derive(Default)]
    pub struct Foo {}
    pub trait Bar {}
    impl Bar for Foo {}

    fn maybe_get_bar(i: u32) -> Option<Box<dyn Bar>> {
        if i % 2 == 0 {
            Some(Box::new(Foo::default()))
            //~^ ERROR: `Box::new(_)` of default value
        } else {
            None
        }
    }

    assert!(maybe_get_bar(2).is_some());
}
