#![allow(unused)]
#![warn(clippy::ref_patterns)]

fn dummy() {}

fn use_in_pattern() {
    let opt = Some(5);
    match opt {
        None => dummy(),
        Some(ref opt) => {
            //~^ ref_patterns
            dummy()
        },
    }
}

fn use_in_binding() {
    let x = 5;
    let ref y = x;
    //~^ ref_patterns
}

fn use_in_parameter(ref x: i32) {}
//~^ ref_patterns

fn main() {}
