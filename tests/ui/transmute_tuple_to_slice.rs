#![allow(unused)]
#![warn(clippy::transmute_tuple_to_slice)]

fn main() {
    let mut data = [1u8, 2, 3];

    unsafe { core::mem::transmute::<_, &mut [u8]>((data.as_mut_ptr(), 2usize)) };

    unsafe { core::mem::transmute::<_, &[u8]>((data.as_ptr(), 2usize)) };

    // Doesn't trigger, uses a const pointer to create a mutable slice
    unsafe { core::mem::transmute::<_, &mut [u8]>((data.as_ptr(), 2usize)) };

    // Doesn't trigger, uses a mut pointer to create a non-mutable slice
    unsafe { core::mem::transmute::<_, &[u8]>((data.as_mut_ptr(), 2usize)) };
}
