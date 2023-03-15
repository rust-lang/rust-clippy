// run-rustfix
#![allow(unused)]
#![warn(clippy::needless_traits_in_scope)]

pub mod useless_trait_in_scope {
    use std::io::Read;

    pub fn warn() -> std::io::Result<usize> {
        let mut b = "The trait is not used explicitely -> 'use std::io::Read' doesn't need to be in scope".as_bytes();
        let mut buffer = [0; 10];
        b.read(&mut buffer)
    }
}

pub mod trait_not_in_scope {
    use std::io::Read as _;

    pub fn ok() -> std::io::Result<usize> {
        let mut b = "The trait is not used explicitely, but 'use std::io::Read' is already not in scope".as_bytes();
        let mut buffer = [0; 10];
        b.read(&mut buffer)
    }
}

pub mod is_not_a_trait {
    mod inner {
        pub struct Read;
    }
    use inner::Read;

    pub fn ok() {
        let _ = Read;
    }
}

// FIXME: when the trait is explicitely used, the lint should not trigger
// pub mod useful_trait_in_scope {
//     use std::io::Read;

//     pub fn ok() -> std::io::Result<usize> {
//         let mut b = "Trait is used explicitely -> 'use std::io::Read' is OK".as_bytes();
//         let mut buffer = [0; 10];
//         Read::read(&mut b, &mut buffer)
//     }
// }

fn main() {
    useless_trait_in_scope::warn();
    trait_not_in_scope::ok();
    is_not_a_trait::ok();
    // useful_trait_in_scope::ok();
}
