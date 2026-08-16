#![warn(clippy::nonnull_unchecked_on_box_ptr)]

use std::ptr::NonNull;

macro_rules! identity {
    ($x:expr) => {
        $x
    };
}

macro_rules! weird {
    ($x:expr) => {{
        let y = 1;
        $x
    }};
}

macro_rules! nonnull_new_unchecked_box_into_raw {
    ($x:expr) => {
        unsafe { NonNull::new_unchecked(Box::into_raw($x)) }
    };
}

macro_rules! nonnull_from_mut_box_leak {
    ($x:expr) => {
        NonNull::from_mut(Box::leak($x))
    };
}

fn identity<T>(x: T) -> T {
    x
}

unsafe fn unsafe_identity<T>(x: T) -> T {
    x
}

fn lint() {
    fn basic() {
        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(Box::into_raw(one))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(Box::leak(one));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(Box::into_raw(identity(one)))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(Box::leak(identity(one)));
        //~^ nonnull_unchecked_on_box_ptr
    }

    fn qualifiers() {
        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            std::ptr::NonNull::new_unchecked(std::boxed::Box::into_raw(one))
        };

        let one = Box::new(1);
        let _ = std::ptr::NonNull::from_mut(std::boxed::Box::leak(one));
        //~^ nonnull_unchecked_on_box_ptr

        {
            use Box as Box2;
            use NonNull as NonNull2;
            let one = Box::new(1);
            let _ = unsafe {
                //~^ nonnull_unchecked_on_box_ptr
                NonNull2::new_unchecked(Box2::into_raw(one))
            };

            let one = Box::new(1);
            let _ = NonNull2::from_mut(Box2::leak(one));
            //~^ nonnull_unchecked_on_box_ptr
        }

        {
            type Box2<T> = Box<T>;
            type NonNull2<T> = NonNull<T>;
            let one = Box::new(1);
            let _ = unsafe {
                //~^ nonnull_unchecked_on_box_ptr
                NonNull2::new_unchecked(Box2::into_raw(one))
            };

            let one = Box::new(1);
            let _ = NonNull2::from_mut(Box2::leak(one));
            //~^ nonnull_unchecked_on_box_ptr
        }
    }

    fn macros() {
        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(Box::into_raw(identity!(one)))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(Box::leak(identity!(one)));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = identity!(unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(Box::into_raw(one))
        });

        let one = Box::new(1);
        let _ = identity!(NonNull::from_mut(Box::leak(one)));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            identity!(NonNull::new_unchecked(Box::into_raw(one)))
        };

        let one = Box::new(1);
        let _ = identity!(NonNull::from_mut(Box::leak(one)));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(identity!(Box::into_raw(one)))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(identity!(Box::leak(one)));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(identity!(Box::into_raw(identity!(one))))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(identity!(Box::leak(identity!(one))));
        //~^ nonnull_unchecked_on_box_ptr

        let one = Box::new(1);
        let _ = unsafe {
            //~^ nonnull_unchecked_on_box_ptr
            NonNull::new_unchecked(Box::into_raw(weird!(one)))
        };

        let one = Box::new(1);
        let _ = NonNull::from_mut(Box::leak(weird!(one)));
        //~^ nonnull_unchecked_on_box_ptr
    }

    fn keep_unsafe_block() {
        let one = Box::new(1);
        let _ = unsafe {
            NonNull::new_unchecked(Box::into_raw(unsafe_identity(one)))
            //~^ nonnull_unchecked_on_box_ptr
        };

        let one = Box::new(1);
        let _ = unsafe {
            identity(NonNull::new_unchecked(Box::into_raw(one)))
            //~^ nonnull_unchecked_on_box_ptr
        };

        let one = Box::new(1);
        let _ = unsafe {
            unsafe_identity(NonNull::new_unchecked(Box::into_raw(one)))
            //~^ nonnull_unchecked_on_box_ptr
        };

        let _ = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(std::num::NonZeroI32::new_unchecked(1))))
            //~^ nonnull_unchecked_on_box_ptr
        };

        let _ = unsafe {
            let one = Box::new(1);
            NonNull::new_unchecked(Box::into_raw(one))
            //~^ nonnull_unchecked_on_box_ptr
        };
    }
}

fn no_lint() {
    fn basic() {
        let one = Box::new(1);
        let _ = NonNull::from_mut(identity(Box::leak(one)));

        let one = Box::new(1);
        let _ = unsafe { NonNull::new_unchecked(identity(Box::into_raw(one))) };
    }

    // We intentionally do not check for this case; if the user has separated the two
    // expressions, it is likely for the good reason that `leaked` will be used later.
    fn does_not_check_expr_init() {
        let one = Box::new(1);
        let leaked = Box::into_raw(one);
        let _ = unsafe { NonNull::new_unchecked(leaked) };

        let one = Box::new(1);
        let leaked = Box::leak(one);
        let _ = NonNull::from_mut(leaked);
    }

    fn macros() {
        let one = Box::new(1);
        let _ = nonnull_new_unchecked_box_into_raw!(one);

        let one = Box::new(1);
        let _ = nonnull_from_mut_box_leak!(one);
    }
}

#[clippy::msrv = "1.97"]
fn msrv_1_97() {
    let one = Box::new(1);
    let _ = unsafe { NonNull::new_unchecked(Box::into_raw(one)) };

    let one = Box::new(1);
    let _ = NonNull::from_mut(Box::leak(one));
    //~^ nonnull_unchecked_on_box_ptr
}

#[clippy::msrv = "1.98"]
fn msrv_1_98() {
    let one = Box::new(1);
    let _ = unsafe {
        //~^ nonnull_unchecked_on_box_ptr
        NonNull::new_unchecked(Box::into_raw(one))
    };

    let one = Box::new(1);
    let _ = NonNull::from_mut(Box::leak(one));
    //~^ nonnull_unchecked_on_box_ptr
}

fn main() {}
