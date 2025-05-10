//@aux-build:../../auxiliary/proc_macros.rs

#![warn(clippy::proper_safety_comment)]
#![allow(clippy::missing_safety_doc, clippy::no_effect)]

extern crate proc_macros;

mod proc_macro_attribute {
    proc_macros::with_span!(
        span

        #[unsafe(no_mangle)]
        struct A;
    );

    proc_macros::with_span!(
        // SAFETY:
        span

        // SAFETY:
        #[derive(Debug)]
        struct B;
    );
}

fn proc_macro_block() {
    proc_macros::with_span!(
        span

        let mut x = unsafe { 0 };
    );

    proc_macros::with_span!(
        span

        {
            // SAFETY:
            x += 1;
        }
    );

    x += 1;
}

mod proc_macro_extern {
    proc_macros::with_span!(
        span

        unsafe extern {
            pub safe fn f1();
            pub unsafe fn f2();
            pub fn f3();
        }
    );

    proc_macros::with_span!(
        span

        extern {
            pub fn g();
        }
    );
}

mod proc_macro_impl {
    unsafe trait A {}
    trait B {}

    proc_macros::with_span!(
        span

        unsafe impl A for () {}
    );

    proc_macros::with_span!(
        // SAFETY:
        span

        // SAFETY:
        impl B for () {}
    );
}

fn main() {}
