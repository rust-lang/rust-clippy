#![warn(clippy::static_mut_vars)]

unsafe extern "C" {
    pub safe static mut EXTERN_VAR: [u8; 5];
}

static mut BOOL_VAR: bool = false;
//~^ static_mut_vars

static mut ISIZE_VAR: isize = 0isize;
//~^ static_mut_vars
static mut I8_VAR: i8 = 0i8;
//~^ static_mut_vars
static mut I16_VAR: i16 = 0i16;
//~^ static_mut_vars
static mut I32_VAR: i32 = 1i32;
//~^ static_mut_vars
static mut I64_VAR: i64 = 1i64;
//~^ static_mut_vars

static mut USIZE_VAR: usize = 0usize;
//~^ static_mut_vars
static mut U8_VAR: u8 = 0u8;
//~^ static_mut_vars
static mut U16_VAR: u16 = 0u16;
//~^ static_mut_vars
static mut U32_VAR: u32 = 0u32;
//~^ static_mut_vars
static mut U64_VAR: u64 = 0u64;
//~^ static_mut_vars

static mut MUT_PTR_VAR: *mut i32 = std::ptr::null_mut();
//~^ static_mut_vars

static VAL: u8 = 0u8;

fn main() {}
