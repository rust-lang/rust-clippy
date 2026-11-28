//@no-rustfix
#![warn(clippy::unit_as_impl_trait)]

// Empty bodies whose closing brace is not alone on its line: no suggestion is made
#[rustfmt::skip]
fn empty_with_inline_comment() -> impl Copy { /* nothing */ }
//~^ unit_as_impl_trait

#[rustfmt::skip]
fn empty_with_comment_and_spaces() -> impl Copy {
    //~^ unit_as_impl_trait
    /* nothing */    }

struct EmptyInImpl;

impl EmptyInImpl {
    #[rustfmt::skip]
    fn empty_with_inline_comment() -> impl Copy { /* nothing */ }
    //~^ unit_as_impl_trait
}

fn main() {}
