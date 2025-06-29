#![warn(clippy::proper_safety_comment)]

//~v ERROR: missing safety comment on `unsafe extern`-block
unsafe extern "C" {
    //~v ERROR: missing safety comment on item in `unsafe extern`-block
    pub safe fn f1();

    //~v ERROR: missing safety comment on item in `unsafe extern`-block
    pub unsafe fn f2();

    //~v ERROR: missing safety comment on item in `unsafe extern`-block
    pub fn f3();
}

//~v ERROR: missing safety comment on `unsafe extern`-block
extern "C" {
    //~v ERROR: missing safety comment on item in `unsafe extern`-block
    pub fn f4();
}

// SAFETY:
unsafe extern "C" {
    // SAFETY:
    pub safe fn g1();

    // SAFETY:
    pub unsafe fn g2();

    // SAFETY:
    pub fn g3();
}

// SAFETY:
extern "C" {
    // SAFETY:
    pub fn g4();
}

// SAFETY:

unsafe extern "C" {
    // SAFETY:

    // ...

    pub safe fn h1();

    // SAFETY:
    // ...

    pub unsafe fn h2();

    // SAFETY:

    pub fn h3();
}

// SAFETY:

// ...

extern "C" {
    // SAFETY:

    pub fn h4();
}

fn main() {}
