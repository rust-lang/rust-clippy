#![warn(clippy::transmute_ptr_to_ref)]
#![allow(clippy::match_single_binding, clippy::unnecessary_cast)]

unsafe fn _ptr_to_ref<T, U>(p: *const T, m: *mut T, o: *const U, om: *mut U) {
    let _: &T = std::mem::transmute(p);
    //~^ ERROR: transmute from a pointer type (`*const T`) to a reference type (`&T`)
    //~| NOTE: `-D clippy::transmute-ptr-to-ref` implied by `-D warnings`
    let _: &T = &*p;

    let _: &mut T = std::mem::transmute(m);
    //~^ ERROR: transmute from a pointer type (`*mut T`) to a reference type (`&mut T`)
    let _: &mut T = &mut *m;

    let _: &T = std::mem::transmute(m);
    //~^ ERROR: transmute from a pointer type (`*mut T`) to a reference type (`&T`)
    let _: &T = &*m;

    let _: &mut T = std::mem::transmute(p as *mut T);
    //~^ ERROR: transmute from a pointer type (`*mut T`) to a reference type (`&mut T`)
    let _ = &mut *(p as *mut T);

    let _: &T = std::mem::transmute(o);
    //~^ ERROR: transmute from a pointer type (`*const U`) to a reference type (`&T`)
    let _: &T = &*(o as *const T);

    let _: &mut T = std::mem::transmute(om);
    //~^ ERROR: transmute from a pointer type (`*mut U`) to a reference type (`&mut T`)
    let _: &mut T = &mut *(om as *mut T);

    let _: &T = std::mem::transmute(om);
    //~^ ERROR: transmute from a pointer type (`*mut U`) to a reference type (`&T`)
    let _: &T = &*(om as *const T);
}

fn _issue1231() {
    struct Foo<'a, T> {
        bar: &'a T,
    }

    let raw = 42 as *const i32;
    let _: &Foo<u8> = unsafe { std::mem::transmute::<_, &Foo<_>>(raw) };
    //~^ ERROR: transmute from a pointer type (`*const i32`) to a reference type (`&_issue

    let _: &Foo<&u8> = unsafe { std::mem::transmute::<_, &Foo<&_>>(raw) };
    //~^ ERROR: transmute from a pointer type (`*const i32`) to a reference type (`&_issue

    type Bar<'a> = &'a u8;
    let raw = 42 as *const i32;
    unsafe { std::mem::transmute::<_, Bar>(raw) };
    //~^ ERROR: transmute from a pointer type (`*const i32`) to a reference type (`&u8`)
}

unsafe fn _issue8924<'a, 'b, 'c>(x: *const &'a u32, y: *const &'b u32) -> &'c &'b u32 {
    match 0 {
        0 => std::mem::transmute(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
        1 => std::mem::transmute(y),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
        2 => std::mem::transmute::<_, &&'b u32>(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
        _ => std::mem::transmute::<_, &&'b u32>(y),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
    }
}

#[clippy::msrv = "1.38"]
unsafe fn _meets_msrv<'a, 'b, 'c>(x: *const &'a u32) -> &'c &'b u32 {
    let a = 0u32;
    let a = &a as *const u32;
    let _: &u32 = std::mem::transmute(a);
    //~^ ERROR: transmute from a pointer type (`*const u32`) to a reference type (`&u32`)
    let _: &u32 = std::mem::transmute::<_, &u32>(a);
    //~^ ERROR: transmute from a pointer type (`*const u32`) to a reference type (`&u32`)
    match 0 {
        0 => std::mem::transmute(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
        _ => std::mem::transmute::<_, &&'b u32>(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
    }
}

#[clippy::msrv = "1.37"]
unsafe fn _under_msrv<'a, 'b, 'c>(x: *const &'a u32) -> &'c &'b u32 {
    let a = 0u32;
    let a = &a as *const u32;
    let _: &u32 = std::mem::transmute(a);
    //~^ ERROR: transmute from a pointer type (`*const u32`) to a reference type (`&u32`)
    let _: &u32 = std::mem::transmute::<_, &u32>(a);
    //~^ ERROR: transmute from a pointer type (`*const u32`) to a reference type (`&u32`)
    match 0 {
        0 => std::mem::transmute(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
        _ => std::mem::transmute::<_, &&'b u32>(x),
        //~^ ERROR: transmute from a pointer type (`*const &u32`) to a reference type (`&&
    }
}

fn main() {}
