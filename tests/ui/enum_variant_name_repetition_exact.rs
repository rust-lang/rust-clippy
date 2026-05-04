//@check-pass
// https://github.com/rust-lang/rust-clippy/issues/16895
#![warn(clippy::enum_variant_names)]

#[allow(unused)]
enum Type {
    Type,
    Builtin(PrimitiveType),
    Func,
}

#[allow(unused)]
enum PrimitiveType {
    Int,
    Float,
}

fn main() {}
