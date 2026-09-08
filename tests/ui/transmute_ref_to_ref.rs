#![warn(clippy::transmute_ptr_to_ptr)]
#![allow(clippy::cast_slice_different_sizes)]

fn main() {
    unsafe {
        let single_u64: &[u64] = &[0xDEAD_BEEF_DEAD_BEEF];
        let bools: &[bool] = unsafe { std::mem::transmute(single_u64) };
        //~^ transmute_ptr_to_ptr

        let a: &[u32] = &[0x12345678, 0x90ABCDEF, 0xFEDCBA09, 0x87654321];
        let b: &[u8] = unsafe { std::mem::transmute(a) };
        //~^ transmute_ptr_to_ptr

        let bytes = &[1u8, 2u8, 3u8, 4u8] as &[u8];
        let alt_slice: &[u32] = unsafe { std::mem::transmute(bytes) };
        //~^ transmute_ptr_to_ptr
    }
}

fn issue16104(make_ptr: fn() -> *const u32) {
    macro_rules! call {
        ($x:expr) => {
            $x()
        };
    }
    macro_rules! take_ref {
        ($x:expr) => {
            &$x
        };
    }

    unsafe {
        let _: *const f32 = std::mem::transmute(call!(make_ptr));
        //~^ transmute_ptr_to_ptr
        let _: &f32 = std::mem::transmute(take_ref!(1u32));
        //~^ transmute_ptr_to_ptr
    }
}

mod msrv_from_utf8_mut {
    #[clippy::msrv = "1.87"]
    fn bytes_to_str_mut_below_msrv(mb: &mut [u8]) {
        let _: &mut str = unsafe { std::mem::transmute(mb) };
        //~^ transmute_ptr_to_ptr
    }

    #[clippy::msrv = "1.88"]
    fn bytes_to_str_mut_after_msrv(mb: &mut [u8]) {
        let _: &mut str = unsafe { std::mem::transmute(mb) };
        //~^ transmute_bytes_to_str
    }
}
