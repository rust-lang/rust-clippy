//@no-rustfix

#![warn(clippy::if_then_some_else_none)]
#![allow(clippy::manual_is_multiple_of)]

mod issue15257 {
    #[derive(Default)]
    pub struct Foo {}
    pub trait Bar {}
    impl Bar for Foo {}

    fn maybe_get_bar(i: u32) -> Option<Box<dyn Bar>> {
        if i % 2 == 0 {
            //~^ if_then_some_else_none
            Some(Box::new(Foo::default()))
        } else {
            None
        }
    }
}
