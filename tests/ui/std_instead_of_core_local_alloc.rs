#![warn(clippy::std_instead_of_alloc)]
#![allow(unused_imports)]

#[rustfmt::skip]
fn issue16695() {
    extern crate alloc;

    use std::collections::VecDeque;
    //~^ std_instead_of_alloc

    {
        use std::vec::Vec;
        //~^ std_instead_of_alloc
    }
}

mod a {
    extern crate alloc as alloc_a;

    fn b() {
        mod c {
            extern crate alloc as alloc_b;

            fn d() {
                fn e() {
                    // This should suggest alloc_b
                    use std::vec::Vec;
                    //~^ std_instead_of_alloc
                }
            }
        }
    }

    // This should suggest alloc_a
    use std::vec::Vec;
    //~^ std_instead_of_alloc
}
