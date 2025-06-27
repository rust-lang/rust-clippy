#![warn(clippy::transmute_ptr_to_ref)]
#![allow(
    clippy::match_single_binding,
    clippy::unnecessary_cast,
    clippy::missing_transmute_annotations
)]

unsafe fn _ptr_to_ref<T, U>(p: *const T, m: *mut T, o: *const U, om: *mut U) {
    unsafe {
        let _: &T = std::mem::transmute(p);
        //~^ transmute_ptr_to_ref
        let _: &T = &*p;

        let _: &mut T = std::mem::transmute(m);
        //~^ transmute_ptr_to_ref
        let _: &mut T = &mut *m;

        let _: &T = std::mem::transmute(m);
        //~^ transmute_ptr_to_ref
        let _: &T = &*m;

        let _: &mut T = std::mem::transmute(p as *mut T);
        //~^ transmute_ptr_to_ref
        let _ = &mut *(p as *mut T);

        let _: &T = std::mem::transmute(o);
        //~^ transmute_ptr_to_ref
        let _: &T = &*(o as *const T);

        let _: &mut T = std::mem::transmute(om);
        //~^ transmute_ptr_to_ref
        let _: &mut T = &mut *(om as *mut T);

        let _: &T = std::mem::transmute(om);
        //~^ transmute_ptr_to_ref
        let _: &T = &*(om as *const T);
    }
}

fn _issue1231() {
    struct Foo<'a, T> {
        bar: &'a T,
    }

    let raw = 42 as *const i32;
    let _: &Foo<u8> = unsafe { std::mem::transmute::<_, &Foo<_>>(raw) };
    //~^ transmute_ptr_to_ref

    let _: &Foo<&u8> = unsafe { std::mem::transmute::<_, &Foo<&_>>(raw) };
    //~^ transmute_ptr_to_ref

    type Bar<'a> = &'a u8;
    let raw = 42 as *const i32;
    unsafe { std::mem::transmute::<_, Bar>(raw) };
    //~^ transmute_ptr_to_ref
}

unsafe fn _issue8924<'a, 'b, 'c>(x: *const &'a u32, y: *const &'b u32) -> &'c &'b u32 {
    unsafe {
        match 0 {
            0 => std::mem::transmute(x),
            //~^ transmute_ptr_to_ref
            1 => std::mem::transmute(y),
            //~^ transmute_ptr_to_ref
            2 => std::mem::transmute::<_, &&'b u32>(x),
            //~^ transmute_ptr_to_ref
            _ => std::mem::transmute::<_, &&'b u32>(y),
            //~^ transmute_ptr_to_ref
        }
    }
}

#[clippy::msrv = "1.38"]
unsafe fn _meets_msrv<'a, 'b, 'c>(x: *const &'a u32) -> &'c &'b u32 {
    unsafe {
        let a = 0u32;
        let a = &a as *const u32;
        let _: &u32 = std::mem::transmute(a);
        //~^ transmute_ptr_to_ref
        let _: &u32 = std::mem::transmute::<_, &u32>(a);
        //~^ transmute_ptr_to_ref
        match 0 {
            0 => std::mem::transmute(x),
            //~^ transmute_ptr_to_ref
            _ => std::mem::transmute::<_, &&'b u32>(x),
            //~^ transmute_ptr_to_ref
        }
    }
}

#[clippy::msrv = "1.37"]
unsafe fn _under_msrv<'a, 'b, 'c>(x: *const &'a u32) -> &'c &'b u32 {
    unsafe {
        let a = 0u32;
        let a = &a as *const u32;
        let _: &u32 = std::mem::transmute(a);
        //~^ transmute_ptr_to_ref
        let _: &u32 = std::mem::transmute::<_, &u32>(a);
        //~^ transmute_ptr_to_ref
        match 0 {
            0 => std::mem::transmute(x),
            //~^ transmute_ptr_to_ref
            _ => std::mem::transmute::<_, &&'b u32>(x),
            //~^ transmute_ptr_to_ref
        }
    }
}

unsafe fn _issue1754() {
    let mut_num_func_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

    let num: u32 = 0;

    // If pointer cast is supported by MSRV.
    unsafe {
        // Not recommended.
        std::mem::transmute::<*mut std::ffi::c_void, &mut Box<dyn FnMut(&u32)>>(mut_num_func_ptr)(&num);
        //~^ transmute_ptr_to_ref

        // Recommended.
        (&mut *mut_num_func_ptr.cast::<Box<dyn FnMut(&u32)>>())(&num);
    }

    // If pointer cast is not supported by MSRV.
    #[clippy::msrv = "1.37"]
    unsafe {
        // Not recommended.
        std::mem::transmute::<*mut std::ffi::c_void, &mut Box<dyn FnMut(&u32)>>(mut_num_func_ptr)(&num);
        //~^ transmute_ptr_to_ref

        // Recommended.
        (&mut *(mut_num_func_ptr as *mut Box<dyn FnMut(&u32)>))(&num);
    };

    let num_func_ptr: *const std::ffi::c_void = std::ptr::null_mut();

    // If pointer cast is supported by MSRV.
    unsafe {
        // Not recommended.
        std::mem::transmute::<*const std::ffi::c_void, &Box<dyn Fn(&u32)>>(num_func_ptr)(&num);
        //~^ transmute_ptr_to_ref

        // Recommended.
        (&*num_func_ptr.cast::<Box<dyn Fn(&u32)>>())(&num);
    }

    // If pointer cast is not supported by MSRV.
    #[clippy::msrv = "1.37"]
    unsafe {
        // Not recommended.
        std::mem::transmute::<*const std::ffi::c_void, &Box<dyn Fn(&u32)>>(num_func_ptr)(&num);
        //~^ transmute_ptr_to_ref

        // Recommended.
        (&*(num_func_ptr as *const Box<dyn Fn(&u32)>))(&num);
    };
}

fn main() {}
