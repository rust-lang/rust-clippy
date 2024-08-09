//@only-target-x86_64
#![allow(clippy::missing_safety_doc)]

pub fn test_f() {
    unsafe { f() };
    //~^ ERROR: this call requires target features
}

pub(crate) fn test_g() {
    unsafe { g() };
    //~^ ERROR: this call requires target features
}

fn test_h() {
    unsafe { h() };
    //~^ ERROR: this call requires target features
}

#[target_feature(enable = "avx2")]
unsafe fn f() -> u32 {
    0
}

#[target_feature(enable = "avx2,pclmulqdq")]
unsafe fn g() -> u32 {
    0
}

#[target_feature(enable = "avx2")]
#[target_feature(enable = "pclmulqdq")]
unsafe fn h() -> u32 {
    0
}
