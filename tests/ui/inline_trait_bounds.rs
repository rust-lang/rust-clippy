#![warn(clippy::inline_trait_bounds)]
#![allow(unused)]

fn inline_foo<T: Clone>() {}
//~^ inline_trait_bounds

// should be ok
fn where_bar<T>()
where
    T: Clone,
{
}

struct InlineFoo<T: Clone>(T);
//~^ inline_trait_bounds

// should be ok
struct WhereBar<T>(T)
where
    T: Clone;

fn main() {}
