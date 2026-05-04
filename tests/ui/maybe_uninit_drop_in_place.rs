//@no-rustfix: lint emits help, not a suggestion (rewrite to `assume_init_drop` is UB if uninit)
#![warn(clippy::maybe_uninit_drop_in_place)]
#![allow(clippy::cast_slice_from_raw_parts)]

use std::mem::{ManuallyDrop, MaybeUninit};

fn pos_basic() {
    let mut x = std::mem::MaybeUninit::<String>::uninit();
    unsafe {
        std::ptr::drop_in_place(&mut x);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_reference_inner() {
    let mut x = MaybeUninit::<&i32>::uninit();
    unsafe {
        std::ptr::drop_in_place(&mut x);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_fully_qualified() {
    let mut x = MaybeUninit::<Vec<u8>>::uninit();
    unsafe {
        std::ptr::drop_in_place(&mut x);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_through_binding() {
    let mut x = MaybeUninit::<String>::uninit();
    let p = &mut x;
    unsafe {
        std::ptr::drop_in_place(p);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_raw_pointer() {
    let mut x = MaybeUninit::<String>::uninit();
    let p: *mut MaybeUninit<String> = &raw mut x;
    unsafe {
        std::ptr::drop_in_place(p);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_generic_inner<T>() {
    let mut x = MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::drop_in_place(&mut x);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_slice_of_maybe_uninit() {
    let mut buf: [MaybeUninit<String>; 4] = [const { MaybeUninit::uninit() }; 4];
    let begin = 0usize;
    let len = 2usize;
    unsafe {
        let slice = std::slice::from_raw_parts_mut(buf.as_mut_ptr().add(begin), len);
        std::ptr::drop_in_place(slice);
        //~^ maybe_uninit_drop_in_place
    }
}

fn pos_array_of_maybe_uninit() {
    let mut buf: [MaybeUninit<String>; 4] = [const { MaybeUninit::uninit() }; 4];
    unsafe {
        std::ptr::drop_in_place(&mut buf);
        //~^ maybe_uninit_drop_in_place
    }
}

fn neg_slice_cast_to_inner() {
    let mut buf: [MaybeUninit<String>; 4] = [const { MaybeUninit::uninit() }; 4];
    let begin = 0usize;
    let length = 2usize;
    unsafe {
        let k1_ptr = buf.as_mut_ptr().add(begin) as *mut String;
        std::ptr::drop_in_place(std::slice::from_raw_parts_mut(k1_ptr, length));
    }
}

fn neg_correct_inner_drop() {
    let mut x = MaybeUninit::new(String::from("hi"));
    unsafe {
        std::ptr::drop_in_place(x.as_mut_ptr());
    }
}

fn neg_assume_init_drop() {
    let mut x = MaybeUninit::new(String::from("hi"));
    unsafe {
        x.assume_init_drop();
    }
}

fn neg_plain_type() {
    let mut s = ManuallyDrop::new(String::from("hello"));
    unsafe {
        std::ptr::drop_in_place(&mut *s);
    }
}

// Don't lint code that the user can't fix.
macro_rules! drop_it {
    ($e:expr) => {
        unsafe { std::ptr::drop_in_place(&mut $e) }
    };
}
fn neg_in_macro() {
    let mut x = MaybeUninit::<String>::uninit();
    drop_it!(x);
}

fn neg_fully_generic<T>(x: &mut T) {
    unsafe {
        std::ptr::drop_in_place(x);
    }
}

fn main() {}
