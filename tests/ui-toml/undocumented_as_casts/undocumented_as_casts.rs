//@aux-build:../../ui/auxiliary/proc_macros.rs
//@revisions: default const_ptr mut_ptr
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/undocumented_as_casts/default
//@[const_ptr] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/undocumented_as_casts/disable_const_ptr
//@[mut_ptr] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/undocumented_as_casts/disable_mut_ptr

#![warn(clippy::undocumented_as_casts)]

fn lint_mut_ptr_without_comment() {
    let p: *mut u32 = &mut 42_u32;
    let _ = p as *mut i32;
    //~[default]^ undocumented_as_casts
    //~[const_ptr]^^ undocumented_as_casts
}

fn lint_const_ptr_without_comment_default_only() {
    let p: *const u32 = &42_u32;
    let _ = p as *const i32;
    //~[default]^ undocumented_as_casts
    //~[mut_ptr]^^ undocumented_as_casts
}

fn allow_with_cast_comment() {
    let p: *mut u32 = &mut 42_u32;
    // CAST: reason for the cast
    let _ = p as *mut i32;

    let q: *const u32 = &42_u32;
    // CAST: reason for the cast
    let _ = q as *const i32;
}

fn main() {}
