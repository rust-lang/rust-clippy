#![feature(type_alias_impl_trait)]
#![warn(clippy::type_complexity)]

fn complex_opaque_bound() -> impl Fn(Vec<Vec<Box<(u32, u32, u32, u32)>>>) {
    //~^ ERROR: very complex type used. Consider factoring parts into `type` definitions
    |_| {}
}

fn main() {}
