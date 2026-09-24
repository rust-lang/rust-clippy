#![warn(clippy::absolute_paths)]
#![allow(unused, clippy::clone_on_copy, clippy::let_and_return)]

// The imports go above this attribute, not between it and the item.
#[derive(Debug)]
struct First;

fn normal() {
    let _x = std::collections::HashMap::<u32, u32>::new();
    //~^ absolute_paths
}

// One `use` per scope, but every occurrence is shortened.
fn repeated() {
    let _x = std::collections::BTreeMap::<u32, u32>::new();
    //~^ absolute_paths
    let _y = std::collections::BTreeMap::<u8, u8>::new();
    //~^ absolute_paths
}

// A module-level `use` could become unused under a disabled feature, so gated
// usages import under the same gate instead. Nothing here is compiled, but the
// `#[cfg(not(test))]` cases below are.
#[cfg(feature = "nonexistent")]
fn gated_fn() {
    let _y = std::collections::BTreeMap::<u32, u32>::new();
}

fn inner_gated() {
    let _a = std::collections::HashSet::<u32>::new();
    //~^ absolute_paths

    #[cfg(feature = "nonexistent")]
    {
        let _b = std::collections::VecDeque::<u32>::new();
    }
}

// A gated path in a *signature*: a body-local `use` would not be in scope for
// the return type, so the import goes before the item with the gate replicated.
#[cfg(not(test))]
fn gated_signature() -> std::collections::BinaryHeap<u32> {
    //~^ absolute_paths
    std::collections::BinaryHeap::new()
    //~^ absolute_paths
}

// The import goes inside the gated block, which keeps its own gate.
fn active_gated_block() {
    #[cfg(not(test))]
    {
        let _c = std::sync::atomic::AtomicUsize::new(0);
        //~^ absolute_paths
    }
}

// An allowed occurrence claims no name and pulls in no import. An expected one is reported, so
// that the expectation is fulfilled, but its suggestion is suppressed, so it must not carry
// another occurrence's import either.
fn lint_levels() {
    #[allow(clippy::absolute_paths)]
    let _a = std::collections::BTreeSet::<u32>::new();

    #[expect(clippy::absolute_paths)]
    let _b = std::collections::VecDeque::<u32>::new();
    let _c = std::collections::VecDeque::<u8>::new();
    //~^ absolute_paths
}

// The import goes into the innermost module, keeping its indentation.
mod nested {
    pub fn f() {
        let _x = std::collections::LinkedList::<u32>::new();
        //~^ absolute_paths
    }
}

// Macro and derive paths written in the source are rewritten too, but not
// attribute macro paths: importing one can take over a built-in attribute of
// the same name, such as `test`.
mod macros {
    fn call_path() {
        let x = 1;
        let _ = core::ptr::addr_of!(x);
        //~^ absolute_paths
    }

    #[derive(core::clone::Clone)]
    //~^ absolute_paths
    struct Derived;

    #[core::prelude::v1::derive(Debug)]
    struct AttributePath;

    // Paths passed to a macro are the user's own, so they are rewritten as usual.
    fn in_arguments() {
        let _ = vec![std::collections::BTreeSet::<u8>::new()];
        //~^ absolute_paths
    }
}

// With the item already imported, only the path is shortened.
mod already_imported {
    use std::collections::HashMap;

    fn f() {
        let _: HashMap<u8, u8> = std::collections::HashMap::new();
        //~^ absolute_paths
    }
}

// Only the trait's path is shortened, not the `<T as` in front of it.
mod qualified {
    fn f() {
        let _ = <i32 as core::clone::Clone>::clone(&0);
        //~^ absolute_paths
    }
}

fn main() {}
