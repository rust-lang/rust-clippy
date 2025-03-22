#![warn(clippy::proper_safety_comment)]

//~v ERROR: missing safety comment on critical attribute
#[unsafe(no_mangle)]
pub fn f1() {}

//~v ERROR: unnecessary safety comment on attribute
// SAFETY: ...
#[warn(clippy::proper_safety_comment)]
#[unsafe(no_mangle)]
//~^ ERROR: missing safety comment on critical attribute
pub fn f2() {}

#[warn(clippy::proper_safety_comment)]
// SAFETY: ...
#[unsafe(no_mangle)]
pub fn f3() {}

fn nested() {
    //~v ERROR: missing safety comment on critical attribute
    #[unsafe(no_mangle)]
    pub fn f4() {}
}

// SAFETY: not detected as unnecessary safety comment due to the procedural macro
#[derive(Debug)]
struct S1;

#[derive(Debug)]
struct S2;

fn main() {}
