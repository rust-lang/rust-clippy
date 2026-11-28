#![warn(clippy::static_mut_vars)]
//@no-rustfix

static mut F32_VAR: f32 = 1.0f32;
//~^ static_mut_vars
static mut F64_VAR: f64 = 1.0f64;
//~^ static_mut_vars

static mut CHAR_VAR: char = 'a';
//~^ static_mut_vars

static mut REF_VAR: &str = "foo";
//~^ static_mut_vars

static mut PTR_VAR: *const i32 = std::ptr::null();
//~^ static_mut_vars

static mut ARR_VAR: [u8; 3] = [0u8, 1u8, 2u8];
//~^ static_mut_vars

static mut TUPLE_VAR: (u8, u8) = (0u8, 0u8);
//~^ static_mut_vars

static mut FN_PTR_VAR: fn(i32) -> i32 = |x| x * x;
//~^ static_mut_vars

fn main() {}
