//@no-rustfix: the shortened name is already taken, so no import is suggested
#![warn(clippy::absolute_paths)]
#![allow(unused, clippy::extra_unused_type_parameters, clippy::swap_with_temporary)]

// Importing both would be `E0252`, so only the first is rewritten.
fn collide() {
    let _a = std::collections::hash_map::Entry::<u32, u32>::Occupied;
    //~^ absolute_paths
    let _b = std::collections::btree_map::Entry::<u32, u32>::Occupied;
    //~^ absolute_paths
}

// Importing `HashMap` here would be `E0255`.
struct HashMap;

fn shadowed_by_item() {
    let _x = std::collections::HashMap::<u32, u32>::new();
    //~^ absolute_paths
}

// The shortened path would resolve to the block-local `BTreeMap`.
fn shadowed_by_block_item() {
    struct BTreeMap;
    let _x = std::collections::BTreeMap::<u32, u32>::new();
    //~^ absolute_paths
}

// Locals, parameters and generic parameters shadow an import, so the shortened
// path would resolve to them instead.
fn shadowed_by_local() {
    let max = 5;
    let _ = std::cmp::max(1, max);
    //~^ absolute_paths
}

fn shadowed_by_param(min: u32) {
    let _ = std::cmp::min(1, min);
    //~^ absolute_paths
}

fn shadowed_by_closure_param() {
    let _ = |swap: u8| std::mem::swap(&mut 1, &mut 2);
    //~^ absolute_paths
}

fn shadowed_by_generic<LinkedList>() {
    let _ = std::collections::LinkedList::<u8>::new();
    //~^ absolute_paths
}

// On a single line, inserting at the start of the line would put the `use`
// outside the module.
#[rustfmt::skip]
mod one_line { pub fn f() { let _ = std::collections::HashSet::<u8>::new(); } }
//~^ absolute_paths

// A renamed import already binds the name.
mod renamed {
    use std::collections::BTreeMap as HashMap;

    fn f() {
        let _ = std::collections::HashMap::<u8, u8>::new();
        //~^ absolute_paths
    }
}

// The import would take the name over from the prelude, changing what every
// plain use of it in the module means.
mod shadows_prelude {
    mod inner {
        pub struct Vec;
        impl Vec {
            pub fn new() -> Self {
                Vec
            }
        }
    }

    fn f() {
        let _: Vec<u8> = Vec::new();
        let _ = crate::shadows_prelude::inner::Vec::new();
        //~^ absolute_paths
    }
}

mod shadows_prelude_macro {
    fn f() {
        core::prelude::rust_2015::panic!("boom");
        //~^ absolute_paths
        panic!("{}", 1);
    }
}

// The same holds for a bare macro call inside a macro's body, which resolves
// where that macro is used.
mod shadows_prelude_macro_in_macro {
    macro_rules! boom {
        () => {
            panic!("{}", 1)
        };
    }

    fn f() {
        core::prelude::rust_2015::panic!("boom");
        //~^ absolute_paths
    }

    fn g() {
        boom!();
    }
}

fn main() {}
