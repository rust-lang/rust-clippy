//@only-target: x86_64
#![warn(clippy::call_missing_target_feature)]
#![allow(clippy::missing_safety_doc)]

pub fn test_f() {
    unsafe { f() };
    //~^ call_missing_target_feature
}

pub(crate) fn test_g() {
    unsafe { g() };
    //~^ ERROR: this call requires target features
}

fn test_h() {
    unsafe { h() };
    //~^ ERROR: this call requires target features
}

unsafe fn test_h_unsafe() {
    unsafe { h() };
    //~^ ERROR: this call requires target features
}

// check if avx2 is available
fn test_runtime_guarded() {
    if is_x86_feature_detected!("avx2") {
        unsafe { f() };
        //~^ ERROR: this call requires target features
    }
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

// checks for false positives. this test should not return errors
#[target_feature(enable = "avx2")]
unsafe fn test_f_valid() {
    unsafe { f() };
}
