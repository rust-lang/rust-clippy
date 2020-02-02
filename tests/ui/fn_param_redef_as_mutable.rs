#![warn(clippy::fn_param_redef_as_mutable)]

fn foobar(a: Vec<bool>) {
    let mut c = a;
}

fn main() {
}
