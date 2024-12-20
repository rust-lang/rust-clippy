#![warn(clippy::proper_safety_comment)]
#![allow(dead_code, unused_unsafe)]

unsafe fn f1() {}

unsafe fn f2() {
    // SAFETY:
    //~^ ERROR: unnecessary safety comment inside block
}

fn main() {
    unsafe {
        // SAFETY:
    }

    unsafe {}
    //~^ ERROR: missing safety comment inside unsafe block

    // SAFETY:
    unsafe {}
    //~^ ERROR: missing safety comment inside unsafe block

    unsafe { // SAFETY:
    }

    unsafe {

        // SAFETY:
    }

    //~v ERROR: missing safety comment inside unsafe block
    unsafe {
        let x = false;
        // SAFETY:
    }

    {
        let x = false;
        // SAFETY:
    }

    //~v ERROR: unnecessary safety comment inside block
    { // SAFETY:
    }

    {
        // SAFETY:
        //~^ ERROR: unnecessary safety comment inside block
    }

    {

        // SAFETY:
        //~^ ERROR: unnecessary safety comment inside block
    }

    println!("{}", unsafe {
        // SAFETY:
        String::from_utf8_unchecked(vec![])
    });

    println!("{}", unsafe { String::from_utf8_unchecked(vec![]) });
    //~^ ERROR: missing safety comment inside unsafe block
}
