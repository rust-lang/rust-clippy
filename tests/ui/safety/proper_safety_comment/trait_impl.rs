#![warn(clippy::proper_safety_comment)]
#![allow(clippy::missing_safety_doc, non_local_definitions)]

unsafe trait UnsafeTrait {}

//~v ERROR: missing safety comment on unsafe impl
unsafe impl UnsafeTrait for u8 {}

// SAFETY: ...
unsafe impl UnsafeTrait for u16 {}

// SAFETY: ...

// ...

unsafe impl UnsafeTrait for u32 {}

//~v ERROR: missing safety comment on unsafe impl
unsafe impl UnsafeTrait for u64 {}

fn impl_in_fn() {
    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl UnsafeTrait for i8 {}

    // SAFETY: ...
    unsafe impl UnsafeTrait for i16 {}
}

mod unsafe_impl {
    unsafe trait A {}
    trait C {}

    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl A for () {}

    // SAFETY:
    unsafe impl A for bool {}

    mod mod_1 {
        unsafe trait B {}
        //~v ERROR: missing safety comment on unsafe impl
        unsafe impl B for () {}
    }

    mod mod_2 {
        unsafe trait B {}
        //
        // SAFETY:
        //
        unsafe impl B for () {}
    }

    //~v ERROR: unnecessary safety comment on impl
    // SAFETY:
    impl C for () {}
}

mod unsafe_impl_from_macro {
    unsafe trait T {}

    macro_rules! no_safety_comment {
        ($t:ty) => {
            unsafe impl T for $t {}
            //~^ ERROR: missing safety comment on unsafe impl
        };
    }

    no_safety_comment!(());

    macro_rules! with_safety_comment {
        ($t:ty) => {
            // SAFETY:
            unsafe impl T for $t {}
        };
    }

    with_safety_comment!(i32);
}

mod unsafe_impl_macro_and_not_macro {
    unsafe trait T {}

    macro_rules! no_safety_comment {
        ($t:ty) => {
            unsafe impl T for $t {}
            //~^ ERROR: missing safety comment on unsafe impl
        };
    }

    no_safety_comment!(());

    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl T for i32 {}

    no_safety_comment!(u32);

    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl T for bool {}
}

#[rustfmt::skip]
mod unsafe_impl_valid_comment {
    unsafe trait SaFety {}
    // SAFETY:
    unsafe impl SaFety for () {}

    unsafe trait MultiLineComment {}
    // The following impl is safe
    // ...
    // SAFETY: reason
    unsafe impl MultiLineComment for () {}

    unsafe trait NoAscii {}
    // SAFETY: 以下のコードは安全です
    unsafe impl NoAscii for () {}

    unsafe trait InlineAndPrecedingComment {}
    // SAFETY:
    /* comment */ unsafe impl InlineAndPrecedingComment for () {}

    unsafe trait BuriedSafety {}
    // Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor
    // incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation
    // ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in
    // reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint
    // occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est
    // laborum.
    // SAFETY:
    // Tellus elementum sagittis vitae et leo duis ut diam quam. Sit amet nulla facilisi
    // morbi tempus iaculis urna. Amet luctus venenatis lectus magna. At quis risus sed vulputate odio
    // ut. Luctus venenatis lectus magna fringilla urna. Tortor id aliquet lectus proin nibh nisl
    // condimentum id venenatis. Vulputate dignissim suspendisse in est ante in nibh mauris cursus.
    unsafe impl BuriedSafety for () {}
}

#[rustfmt::skip]
mod unsafe_impl_invalid_comment {
    unsafe trait NoComment {}

    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl NoComment for () {}

    unsafe trait InlineComment {}

    //~v ERROR: missing safety comment on unsafe impl
    /* SAFETY: */ unsafe impl InlineComment for () {}

    unsafe trait TrailingComment {}

    //~v ERROR: missing safety comment on unsafe impl
    unsafe impl TrailingComment for () {} // SAFETY:

    unsafe trait Interference {}
    // SAFETY:
    const BIG_NUMBER: i32 = 1000000;
    unsafe impl Interference for () {}
    //~^ ERROR: missing safety comment on unsafe impl

    unsafe trait MultiLineBlockComment {}
    /* This is a description
     * Safety: */
    unsafe impl MultiLineBlockComment for () {}
    //~^ ERROR: missing safety comment on unsafe impl
}

fn main() {}
