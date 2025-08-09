#![warn(clippy::cast_slice_from_raw_parts)]

fn require_raw_slice_ptr<T>(_: *const [T]) {}

#[allow(unused_imports, unused_unsafe)]
fn main() {
    let mut vec = vec![0u8; 1];
    let ptr: *const u8 = vec.as_ptr();
    let mptr = vec.as_mut_ptr();
    let _: *const [u8] = unsafe { std::slice::from_raw_parts(ptr, 1) as *const [u8] };
    //~^ cast_slice_from_raw_parts
    let _: *const [u8] = unsafe { std::slice::from_raw_parts_mut(mptr, 1) as *mut [u8] };
    //~^ cast_slice_from_raw_parts
    let _: *const [u8] = unsafe { std::slice::from_raw_parts(ptr, 1) } as *const [u8];
    //~^ cast_slice_from_raw_parts
    {
        use core::slice;
        let _: *const [u8] = unsafe { slice::from_raw_parts(ptr, 1) } as *const [u8];
        //~^ cast_slice_from_raw_parts
        use slice as one;
        let _: *const [u8] = unsafe { one::from_raw_parts(ptr, 1) } as *const [u8];
        //~^ cast_slice_from_raw_parts
    }
    {
        use std::slice;
        let _: *const [u8] = unsafe { slice::from_raw_parts(ptr, 1) } as *const [u8];
        //~^ cast_slice_from_raw_parts
        use slice as one;
        let _: *const [u8] = unsafe { one::from_raw_parts(ptr, 1) } as *const [u8];
        //~^ cast_slice_from_raw_parts
    }

    // implicit cast
    {
        let _: *const [u8] = unsafe { std::slice::from_raw_parts(ptr, 1) };
        //~^ cast_slice_from_raw_parts
        let _: *mut [u8] = unsafe { std::slice::from_raw_parts_mut(mptr, 1) };
        //~^ cast_slice_from_raw_parts
        require_raw_slice_ptr(unsafe { std::slice::from_raw_parts(ptr, 1) });
        //~^ cast_slice_from_raw_parts
    }
}
