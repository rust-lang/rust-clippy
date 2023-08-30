#![allow(clippy::unnecessary_cast, clippy::useless_vec)]

fn main() {
    let vec = vec![b'a', b'b', b'c'];
    let ptr = vec.as_ptr();

    let offset_u8 = 1_u8;
    let offset_usize = 1_usize;
    let offset_isize = 1_isize;

    unsafe {
        let _ = ptr.offset(offset_usize as isize);
        //~^ ERROR: use of `offset` with a `usize` casted to an `isize`
        //~| NOTE: `-D clippy::ptr-offset-with-cast` implied by `-D warnings`
        let _ = ptr.offset(offset_isize as isize);
        let _ = ptr.offset(offset_u8 as isize);

        let _ = ptr.wrapping_offset(offset_usize as isize);
        //~^ ERROR: use of `wrapping_offset` with a `usize` casted to an `isize`
        let _ = ptr.wrapping_offset(offset_isize as isize);
        let _ = ptr.wrapping_offset(offset_u8 as isize);
    }
}
