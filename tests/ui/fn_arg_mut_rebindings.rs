//@aux-build:proc_macros.rs

#![warn(clippy::fn_arg_mut_rebindings)]

extern crate proc_macros;
use proc_macros::external;

external! {
    fn external_macros_rebinding(x: bool, y: bool) {
        let mut x = x;
    }
}

fn fn_body_rebinding(x: bool) {
    //~^ fn_arg_mut_rebindings
    let mut x = x;
}

fn branch_rebinding(x: bool, m: u32) {
    if x {
        let mut m = m;
    }
}

fn arm_rebinding(x: Option<u32>, m: u32) {
    match x {
        Some(_) => {
            let mut m = m;
        },
        None => {
            let mut m = 1;
        },
    }
}

fn inner_block_rebinding(x: bool) {
    {
        let mut x = x;
    }
}

fn shadowed_rebinding(x: bool) {
    let x = 0;
    let mut x = x;
}

trait MyTrait {
    fn provided_fn_rebinding(&self, x: bool) {
        //~^ fn_arg_mut_rebindings
        let mut x = x;
    }

    fn inherent_fn_rebinding(&self, x: bool);
}

struct MyStruct;

impl MyStruct {
    fn impl_fn_rebinding(&self, x: bool) {
        //~^ fn_arg_mut_rebindings
        let mut x = x;
    }
}

impl MyTrait for MyStruct {
    fn inherent_fn_rebinding(&self, x: bool) {
        let mut x = x;
    }
}

fn main() {
    // test code goes here
}
