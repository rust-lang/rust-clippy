#![warn(clippy::ref_as_ptr)]
#![allow(clippy::unnecessary_mut_passed)]

fn main() {
    let _ = &1u8 as *const _; //~ ref_as_ptr
    let _ = &2u32 as *const u32; //~ ref_as_ptr
    let _ = &3.0f64 as *const f64; //~ ref_as_ptr

    let _ = &4 as *const _ as *const f32; //~ ref_as_ptr
    let _ = &5.0f32 as *const f32 as *const u32; //~ ref_as_ptr

    let _ = &mut 6u8 as *const _; //~ ref_as_ptr
    let _ = &mut 7u32 as *const u32; //~ ref_as_ptr
    let _ = &mut 8.0f64 as *const f64; //~ ref_as_ptr

    let _ = &mut 9 as *const _ as *const f32; //~ ref_as_ptr
    let _ = &mut 10.0f32 as *const f32 as *const u32; //~ ref_as_ptr

    let _ = &mut 11u8 as *mut _; //~ ref_as_ptr
    let _ = &mut 12u32 as *mut u32; //~ ref_as_ptr
    let _ = &mut 13.0f64 as *mut f64; //~ ref_as_ptr

    let _ = &mut 14 as *mut _ as *const f32; //~ ref_as_ptr
    let _ = &mut 15.0f32 as *mut f32 as *const u32; //~ ref_as_ptr

    let _ = &1u8 as *const _; //~ ref_as_ptr
    let _ = &2u32 as *const u32; //~ ref_as_ptr
    let _ = &3.0f64 as *const f64; //~ ref_as_ptr

    let _ = &4 as *const _ as *const f32; //~ ref_as_ptr
    let _ = &5.0f32 as *const f32 as *const u32; //~ ref_as_ptr

    let val = 1;
    let _ = &val as *const _; //~ ref_as_ptr
    let _ = &val as *const i32; //~ ref_as_ptr

    let _ = &val as *const _ as *const f32; //~ ref_as_ptr
    let _ = &val as *const i32 as *const f64; //~ ref_as_ptr

    let mut val: u8 = 2;
    let _ = &mut val as *mut u8; //~ ref_as_ptr
    let _ = &mut val as *mut _; //~ ref_as_ptr

    let _ = &mut val as *const u8; //~ ref_as_ptr
    let _ = &mut val as *const _; //~ ref_as_ptr

    let _ = &mut val as *const u8 as *const f64; //~ ref_as_ptr
    let _: *const Option<u8> = &mut val as *const _ as *const _;
    //~^ ref_as_ptr

    let _ = &std::array::from_fn(|i| i * i) as *const [usize; 7];
    //~^ ref_as_ptr
    let _ = &mut std::array::from_fn(|i| i * i) as *const [usize; 8];
    //~^ ref_as_ptr
    let _ = &mut std::array::from_fn(|i| i * i) as *mut [usize; 9];
    //~^ ref_as_ptr
}

#[clippy::msrv = "1.75"]
fn _msrv_1_75() {
    let val = &42_i32;
    let mut_val = &mut 42_i32;

    // `std::ptr::from_{ref, mut}` was stabilized in 1.76. Do not lint this
    let _ = val as *const i32;
    let _ = mut_val as *mut i32;
}

#[clippy::msrv = "1.76"]
fn _msrv_1_76() {
    let val = &42_i32;
    let mut_val = &mut 42_i32;

    let _ = val as *const i32; //~ ref_as_ptr
    let _ = mut_val as *mut i32; //~ ref_as_ptr
}

fn foo(val: &[u8]) {
    let _ = val as *const _; //~ ref_as_ptr
    let _ = val as *const [u8]; //~ ref_as_ptr
}

fn bar(val: &mut str) {
    let _ = val as *mut _; //~ ref_as_ptr
    let _ = val as *mut str; //~ ref_as_ptr
}

struct X<'a>(&'a i32);

impl<'a> X<'a> {
    fn foo(&self) -> *const i64 {
        self.0 as *const _ as *const _ //~ ref_as_ptr
    }

    fn bar(&mut self) -> *const i64 {
        self.0 as *const _ as *const _ //~ ref_as_ptr
    }
}

struct Y<'a>(&'a mut i32);

impl<'a> Y<'a> {
    fn foo(&self) -> *const i64 {
        self.0 as *const _ as *const _ //~ ref_as_ptr
    }

    fn bar(&mut self) -> *const i64 {
        self.0 as *const _ as *const _ //~ ref_as_ptr
    }

    fn baz(&mut self) -> *const i64 {
        self.0 as *mut _ as *mut _ //~ ref_as_ptr
    }
}
