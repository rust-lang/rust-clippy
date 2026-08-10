//@aux-build:proc_macros.rs
//@check-pass
#![warn(clippy::redundant_field_names)]

#[macro_use]
extern crate proc_macros;

struct S {
    field: usize,
}

fn main() {
    let field = 1;

    // Proc macros may preserve the input span for generated fields.
    external! {
        let _ = S { $(field: field) };
    }
}
