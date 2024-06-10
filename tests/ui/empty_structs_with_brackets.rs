//@aux-build:proc_macros.rs
#![deny(clippy::empty_structs_with_brackets)]
#![expect(clippy::no_effect)]

extern crate proc_macros;

use core::hint::black_box;
use proc_macros::{external, inline_macros, with_span};

pub struct Pub1();
pub struct Pub2 {} //~ empty_structs_with_brackets
#[non_exhaustive]
pub struct Pub3 {} //~ empty_structs_with_brackets

#[inline_macros]
fn main() {
    // Tuple struct
    {
        struct S1(); //~ empty_structs_with_brackets
        struct S2(u32);
        struct S3(); //~ empty_structs_with_brackets
        struct S4();

        let S3() = S3();
        let S3 {} = S3 {};
        S3() = S3();
        S3 {} = S3 {};

        black_box(S4);
    }
    // Structs
    {
        struct S1 {} //~ empty_structs_with_brackets
        struct S2 {
            x: u32,
        }
        struct S3 {} //~ empty_structs_with_brackets

        let S3 {} = S3 {};
        S3 {} = S3 {};
    }
    // Cfg fields and comments
    {
        struct S1 {
            #[cfg(any())]
            x: u32,
        }
        struct S2(#[cfg(any())] u32);
        struct S3 {
            // intentionally empty
        }
        struct S4(/* intentionally empty */);
    }
    // With generics
    {
        //~v empty_structs_with_brackets
        struct S1()
        where
            u32: Sized;
        //~v empty_structs_with_brackets
        struct S2<const N: usize>();
        //~v empty_structs_with_brackets
        struct S3<const N: usize>()
        where
            u32: Sized;

        //~vvv empty_structs_with_brackets
        struct S4
        where
            u32: Sized, {}
        //~v empty_structs_with_brackets
        struct S5<const N: usize> {}
        //~vvv empty_structs_with_brackets
        struct S6<const N: usize>
        where
            u32: Sized, {}
    }
    // External macro declarations
    {
        external! {
            struct S1();
            struct S2 {}
        }
        with_span! {
            sp
            struct S3();
            struct S4 {}
        }
    }
    // External macro uses
    {
        struct S1();
        struct S2();
        struct S3(); //~ empty_structs_with_brackets
        struct S4 {} //~ empty_structs_with_brackets

        external!({
            S1();
            S3 {};
            S4 {};
        });
        with_span!(sp {
            S2();
            S3 {};
            S4 {};
        });
    }
    // Macro declarations
    {
        inline! {
            struct S1(); //~ empty_structs_with_brackets
            struct S2 {} //~ empty_structs_with_brackets
            struct $S3();
            struct $S4 {};
            struct S5$(<const N: usize>)();
            struct S6() $(where u32: Sized);
            struct S7($());
        }
    }
    // Macro uses
    {
        struct S1();
        struct S2();
        struct S3(); //~ empty_structs_with_brackets
        struct S4 {} //~ empty_structs_with_brackets

        inline!({
            $S1();
            S2($());
            S3();
            let $S3 {} = $S3 {};
            $S4 {};
        });
    }
}
