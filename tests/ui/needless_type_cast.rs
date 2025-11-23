#![warn(clippy::needless_type_cast)]
#![allow(clippy::no_effect, clippy::unnecessary_cast, unused)]

fn main() {
    // Should lint: let binding always cast to i32
    let count: u8 = 10;
    //~^ needless_type_cast
    let _a = count as i32 + 5;
    let _b = count as i32 * 2;

    // Should lint: let binding always cast to u64
    let value: u16 = 20;
    //~^ needless_type_cast
    let _x = value as u64;
    let _y = value as u64 + 100;

    // Should NOT lint: binding used without cast
    let mixed_use: u16 = 30;
    let _p = mixed_use;
    let _q = mixed_use as u32;

    // Should NOT lint: binding cast to different types
    let different: u8 = 5;
    let _m = different as u16;
    let _n = different as u32;

    // Should NOT lint: binding not cast at all
    let plain: i32 = 100;
    let _r = plain + 1;
    let _s = plain * 2;

    // Should NOT lint: inferred type with literal suffix (suggestion doesn't fix the literal)
    let inferred = 42u8;
    let _t = inferred as i64;
    let _u = inferred as i64 + 10;

    // Should lint: single usage that is a cast
    let single: u8 = 1;
    //~^ needless_type_cast
    let _v = single as usize;
}

fn test_multiple_bindings() {
    // Should lint: both bindings always cast
    let width: u8 = 10;
    //~^ needless_type_cast
    let height: u8 = 20;
    //~^ needless_type_cast
    let _area = (width as u32) * (height as u32);
}

fn test_no_usage() {
    // Should NOT lint: binding never used
    let _unused: u16 = 30;
}
