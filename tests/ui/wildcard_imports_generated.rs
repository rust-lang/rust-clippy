//@aux-build:wildcard_imports_proc_macro.rs
//@check-pass

#![warn(clippy::wildcard_imports)]
#![allow(dead_code)]

extern crate wildcard_imports_proc_macro;

mod fn_mod {
    pub fn foo() {}
}

#[derive(wildcard_imports_proc_macro::WildcardImport)]
struct ProcMacroGenerated;

struct AutomaticallyDerived;

#[automatically_derived]
impl AutomaticallyDerived {
    fn import() {
        use crate::fn_mod::*;
        foo();
    }
}

fn main() {}
