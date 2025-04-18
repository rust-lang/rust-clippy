//@aux-build:extern_fake_libc.rs
#![warn(clippy::unnecessary_cast)]
#![allow(
    clippy::borrow_as_ptr,
    clippy::multiple_bound_locations,
    clippy::no_effect,
    clippy::nonstandard_macro_braces,
    clippy::unnecessary_operation,
    clippy::double_parens,
    nonstandard_style,
    unused
)]

extern crate extern_fake_libc;

type PtrConstU8 = *const u8;
type PtrMutU8 = *mut u8;

fn owo<T>(ptr: *const T) -> *const T {
    ptr as *const T
    //~^ unnecessary_cast
}

fn uwu<T, U>(ptr: *const T) -> *const U {
    ptr as *const U
}

mod fake_libc {
    type pid_t = i32;
    pub unsafe fn getpid() -> pid_t {
        pid_t::from(0)
    }
    // Make sure a where clause does not break it
    pub fn getpid_SAFE_TRUTH<T: Clone>(t: &T) -> pid_t
    where
        T: Clone,
    {
        t;
        unsafe { getpid() }
    }
}

fn aaa() -> ::std::primitive::u32 {
    0
}

use std::primitive::u32 as UnsignedThirtyTwoBitInteger;

fn bbb() -> UnsignedThirtyTwoBitInteger {
    0
}

#[rustfmt::skip]
fn main() {
    // Test cast_unnecessary
    1i32 as i32;
    //~^ unnecessary_cast
    1f32 as f32;
    //~^ unnecessary_cast
    false as bool;
    //~^ unnecessary_cast
    &1i32 as &i32;

    -1_i32 as i32;
    //~^ unnecessary_cast
    - 1_i32 as i32;
    //~^ unnecessary_cast
    -1f32 as f32;
    //~^ unnecessary_cast
    1_i32 as i32;
    //~^ unnecessary_cast
    1_f32 as f32;
    //~^ unnecessary_cast

    let _: *mut u8 = [1u8, 2].as_ptr() as *const u8 as *mut u8;
    //~^ unnecessary_cast

    [1u8, 2].as_ptr() as *const u8;
    //~^ unnecessary_cast
    [1u8, 2].as_ptr() as *mut u8;
    [1u8, 2].as_mut_ptr() as *mut u8;
    //~^ unnecessary_cast
    [1u8, 2].as_mut_ptr() as *const u8;
    [1u8, 2].as_ptr() as PtrConstU8;
    [1u8, 2].as_ptr() as PtrMutU8;
    [1u8, 2].as_mut_ptr() as PtrMutU8;
    [1u8, 2].as_mut_ptr() as PtrConstU8;
    let _: *const u8 = [1u8, 2].as_ptr() as _;
    let _: *mut u8 = [1u8, 2].as_mut_ptr() as _;
    let _: *const u8 = [1u8, 2].as_ptr() as *const _;
    let _: *mut u8 = [1u8, 2].as_mut_ptr() as *mut _;

    owo::<u32>([1u32].as_ptr()) as *const u32;
    //~^ unnecessary_cast
    uwu::<u32, u8>([1u32].as_ptr()) as *const u8;
    //~^ unnecessary_cast
    // this will not lint in the function body even though they have the same type, instead here
    uwu::<u32, u32>([1u32].as_ptr()) as *const u32;
    //~^ unnecessary_cast

    // macro version
    macro_rules! foo {
        ($a:ident, $b:ident) => {
            #[allow(unused)]
            pub fn $a() -> $b {
                1 as $b
            }
        };
    }
    foo!(a, i32);
    foo!(b, f32);
    foo!(c, f64);

    // do not lint cast from cfg-dependant type
    let x = 0 as std::ffi::c_ulong;
    let y = x as u64;
    let x: std::ffi::c_ulong = 0;
    let y = x as u64;

    // do not lint cast to cfg-dependant type
    let x = 1 as std::os::raw::c_char;
    let y = x as u64;

    // do not lint cast to alias type
    1 as I32Alias;
    &1 as &I32Alias;
    // or from
    let x: I32Alias = 1;
    let y = x as u64;
    fake_libc::getpid_SAFE_TRUTH(&0u32) as i32;
    extern_fake_libc::getpid_SAFE_TRUTH() as i32;
    let pid = unsafe { fake_libc::getpid() };
    pid as i32;
    aaa() as u32;
    //~^ unnecessary_cast
    let x = aaa();
    aaa() as u32;
    //~^ unnecessary_cast
    // Will not lint currently.
    bbb() as u32;
    let x = bbb();
    bbb() as u32;

    let i8_ptr: *const i8 = &1;
    let u8_ptr: *const u8 = &1;

    // cfg dependant pointees
    i8_ptr as *const std::os::raw::c_char;
    u8_ptr as *const std::os::raw::c_char;

    // type aliased pointees
    i8_ptr as *const std::ffi::c_char;
    u8_ptr as *const std::ffi::c_char;

    // issue #9960
    macro_rules! bind_var {
        ($id:ident, $e:expr) => {{
            let $id = 0usize;
            let _ = $e != 0usize;
            let $id = 0isize;
            let _ = $e != 0usize;
        }}
    }
    bind_var!(x, (x as usize) + 1);
}

type I32Alias = i32;

mod fixable {
    #![allow(dead_code)]

    fn main() {
        // casting integer literal to float is unnecessary
        100 as f32;
        //~^ unnecessary_cast
        100 as f64;
        //~^ unnecessary_cast
        100_i32 as f64;
        //~^ unnecessary_cast
        let _ = -100 as f32;
        //~^ unnecessary_cast
        let _ = -100 as f64;
        //~^ unnecessary_cast
        let _ = -100_i32 as f64;
        //~^ unnecessary_cast
        100. as f32;
        //~^ unnecessary_cast
        100. as f64;
        //~^ unnecessary_cast
        // Should not trigger
        #[rustfmt::skip]
        let v = vec!(1);
        &v as &[i32];
        0x10 as f32;
        0o10 as f32;
        0b10 as f32;
        0x11 as f64;
        0o11 as f64;
        0b11 as f64;

        1 as u32;
        //~^ unnecessary_cast
        0x10 as i32;
        //~^ unnecessary_cast
        0b10 as usize;
        //~^ unnecessary_cast
        0o73 as u16;
        //~^ unnecessary_cast
        1_000_000_000 as u32;
        //~^ unnecessary_cast

        1.0 as f64;
        //~^ unnecessary_cast
        0.5 as f32;
        //~^ unnecessary_cast

        1.0 as u16;

        let _ = -1 as i32;
        //~^ unnecessary_cast
        let _ = -1.0 as f32;
        //~^ unnecessary_cast

        let _ = 1 as I32Alias;
        let _ = &1 as &I32Alias;

        let x = 1i32;
        let _ = &(x as i32);
        //~^ unnecessary_cast
    }

    type I32Alias = i32;

    fn issue_9380() {
        let _: i32 = -(1) as i32;
        //~^ unnecessary_cast
        let _: f32 = -(1) as f32;
        let _: i64 = -(1) as i64;
        //~^ unnecessary_cast
        let _: i64 = -(1.0) as i64;

        let _ = -(1 + 1) as i64;
    }

    fn issue_9563() {
        let _: f64 = (-8.0 as f64).exp();
        //~^ unnecessary_cast
        #[allow(ambiguous_negative_literals)]
        let _: f64 = -(8.0 as f64).exp(); // should suggest `-8.0_f64.exp()` here not to change code behavior
        //
        //~^^ unnecessary_cast
    }

    fn issue_9562_non_literal() {
        fn foo() -> f32 {
            0.
        }

        let _num = foo() as f32;
        //~^ unnecessary_cast
    }

    fn issue_9603() {
        let _: f32 = -0x400 as f32;
    }

    // Issue #11968: The suggestion for this lint removes the parentheses and leave the code as
    // `*x.pow(2)` which tries to dereference the return value rather than `x`.
    fn issue_11968(x: &usize) -> usize {
        (*x as usize).pow(2)
        //~^ unnecessary_cast
    }

    fn issue_14366(i: u32) {
        // Do not remove the cast if it helps determining the type
        let _ = ((1.0 / 8.0) as f64).powf(i as f64);

        // But remove useless casts anyway
        let _ = (((1.0 / 8.0) as f64) as f64).powf(i as f64);
        //~^ unnecessary_cast
    }

    fn ambiguity() {
        pub trait T {}
        impl T for u32 {}
        impl T for String {}
        fn f(_: impl T) {}

        f((1 + 2) as u32);
        f((1 + 2u32) as u32);
        //~^ unnecessary_cast
    }

    fn with_blocks(a: i64, b: i64, c: u64) {
        let threshold = if c < 10 { a } else { b };
        let _ = threshold as i64;
        //~^ unnecessary_cast
    }

    fn with_prim_ty() {
        let threshold = 20;
        let threshold = if threshold == 0 {
            i64::MAX
        } else if threshold <= 60 {
            10
        } else {
            0
        };
        let _ = threshold as i64;
        //~^ unnecessary_cast
    }
}
