#![warn(clippy::enum_variant_names)]
// https://github.com/rust-lang/rust-clippy/issues/16895

#[allow(unused)]
enum Type {
    Type,
    //~^ enum_variant_names
    Builtin(PrimitiveType),
    Func,
}

#[allow(unused)]
enum PrimitiveType {
    Int,
    Float,
}

fn main() {}
