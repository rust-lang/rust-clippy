#![allow(unused)]
#![warn(clippy::ref_patterns)]

fn use_in_pattern() {
    let opt = Some(5);
    match opt {
        None => {},
        Some(ref opt) => {},
        //~^ ref_patterns
    }
}

fn use_in_binding() {
    let x = 5;
    let ref y = x;
    //~^ ref_patterns
}

fn use_in_parameter(ref x: i32) {}
//~^ ref_patterns

fn issue_16145(bar: &[u8]) -> u8 {
    if let [a, b, ref _rest @ .., c, d, e] = *bar {
        a + b + c + d + e
    } else {
        0
    }
}

fn main() {}
