#![deny(clippy::redundant_clone)]

use core::borrow::Borrow;
use core::hint::black_box;
use core::mem::ManuallyDrop;
use core::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

fn main() {
    {
        let x = black_box(String::new());
        let _ = x.clone(); //~ redundant_clone
        black_box(&x);
        black_box(x);
    }
    {
        let x = black_box(String::new()).clone(); //~ redundant_clone
        black_box(&x);
        black_box(x);
    }
    {
        let x = black_box(String::new());
        drop(x.clone()); //~ redundant_clone
        black_box(&x);
        black_box(x);
    }
    {
        let x = black_box(String::new());
        let y = x.clone(); //~ redundant_clone
        drop(x);
        black_box(&y);
        black_box(y);
    }
    {
        let x = black_box(String::new());
        let _y = x.clone(); //~ redundant_clone
        black_box(&x);
        black_box(x);
    }
    {
        let x = black_box(String::new());
        let y = x.clone(); //~ redundant_clone
        black_box(&y);
        black_box(y);
    }
    {
        let x = black_box(String::new());
        black_box(x.clone()); //~ redundant_clone
    }
    {
        let x = black_box(String::new());
        let y = x.clone(); //~ redundant_clone
        black_box(x);
    }
    {
        let x = black_box(String::new());
        black_box(&mut x.clone()); //~ redundant_clone
    }
    {
        let x = black_box(String::new());
        let mut y = x.clone(); //~ redundant_clone
        black_box(&mut y);
    }
    {
        let mut x = black_box(String::new());
        let _y = x.clone(); //~ redundant_clone
        black_box(&mut x);
    }
    {
        let x = black_box(String::new());
        black_box(x.clone());
        black_box(&x);
    }
    {
        let x = black_box(String::new());
        black_box(x.clone());
        black_box(x);
    }
    {
        let x = black_box(String::new());
        let y = x.clone();
        black_box(x);
        black_box(&y);
    }
    {
        let x = black_box(String::new());
        let y = x.clone();
        black_box(x);
        black_box(y);
    }
    {
        let x = black_box(String::new());
        black_box(&mut x.clone());
        black_box(&x);
    }
    {
        let x = black_box(String::new());
        black_box(&mut x.clone());
        black_box(x);
    }
    {
        let mut x = black_box(String::new());
        let y = x.clone();
        black_box(&mut x);
        black_box(&y);
    }
    {
        let mut x = black_box(String::new());
        let y = x.clone();
        black_box(&mut x);
        black_box(y);
    }
    {
        let x = black_box(String::new());
        let y = if black_box(true) {
            x.clone() //~ redundant_clone
        } else {
            black_box(String::new())
        };
        black_box(y);
    }
    {
        let x = black_box(String::new());
        let y = if black_box(true) {
            x.clone()
        } else {
            black_box(String::new())
        };
        black_box((y, &x));
    }
    {
        let x = black_box(String::new());
        black_box({
            let x = &x;
            x.clone() //~ redundant_clone
        });
    }
    {
        let x = black_box(String::new());
        black_box({
            let x = &x;
            x.clone()
        });
        black_box(x);
    }
    {
        for _ in 0..10 {
            let x = black_box(String::new());
            black_box(x.clone()); //~ redundant_clone
        }
    }
    {
        for _ in 0..10 {
            let mut x = black_box(String::new());
            black_box(x.clone());
            black_box(&mut x);
        }
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            let _y = x.clone(); //~ redundant_clone
            black_box(&mut x);
        }
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            black_box(x.clone());
        }
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            let y = x.clone();
            black_box((&y, &mut x));
        }
    }
    {
        let x = black_box(String::new());
        let y = x.clone();
        let x = &x;
        black_box(y);
        black_box(x);
    }
    {
        let x = black_box(String::new());
        let y = x.clone();
        let z = x.clone(); //~ redundant_clone
        black_box((y, &x));
    }
    {
        let x = black_box(String::new());
        let y = x.clone(); //~ redundant_clone
        let z = x.clone();
        black_box((z, &x));
    }
    {
        let x = black_box(String::new());
        let y = x.clone(); //~ redundant_clone
        let z = y.clone();
        black_box((z, &y));
    }
    {
        let x = black_box((String::new(), 0));
        black_box(x.0.clone()); //~ redundant_clone
    }
    {
        let x = black_box((String::new(), 0));
        let _y = x.0.clone(); //~ redundant_clone
        black_box(&x.0);
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.0.clone(), 0)); //~ redundant_clone
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.0.clone(), 0)); //~ redundant_clone
        black_box(&x.1);
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.0.clone(), 0));
        black_box(&x.0);
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.0.clone(), 0));
        black_box(&x);
    }
    {
        let x = black_box((String::new(), 0));
        black_box(x.clone()); //~ redundant_clone
    }
    {
        let x = black_box((String::new(), 0));
        let _y = x.clone(); //~ redundant_clone
        black_box(&x.1);
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.clone(), &x.1));
    }
    {
        let x = black_box((String::new(), 0));
        black_box((x.clone(), &x.0));
    }
    {
        let x = black_box((String::new(), 0));
        let y = x.clone();
        let x = &x.1;
        black_box((y, x));
    }
    {
        #[derive(Clone)]
        struct X {
            x: String,
            y: (String, String),
            z: u32,
        }
        let x = black_box(X {
            x: String::new(),
            y: (String::new(), String::new()),
            z: 0,
        });
        black_box((
            x.clone(),   //~ redundant_clone
            x.x.clone(), //~ redundant_clone
            x.y.0.clone(),
            x.y.1.clone(),
            x.y.clone(), //~ redundant_clone
        ));

        let x = black_box(X {
            x: String::new(),
            y: (String::new(), String::new()),
            z: 0,
        });
        black_box((
            x.clone(),
            x.x.clone(), //~ redundant_clone
            x.y.0.clone(),
            x.y.1.clone(),
            x.y.clone(),
        ));
        black_box(&x.y);
    }
    {
        fn f1<T: Clone>(x: T) -> T {
            x.clone() //~ redundant_clone
        }
        fn f2<T: Clone>(x: T) -> T {
            drop(x.clone()); //~ redundant_clone
            x
        }
        fn f3<T: Clone>(x: T) -> T {
            black_box(x.clone());
            x
        }
        fn f4(x: String, y: String) -> String {
            let z = if black_box(true) {
                x.clone()
            } else {
                y.clone() //~ redundant_clone
            };
            black_box(z);
            x
        }
    }
    {
        let mut x = black_box(String::new());
        let mut y = x.clone(); //~ redundant_clone
        black_box(&mut y);
        x = black_box(String::new());
        black_box(x);
    }
    {
        let mut x = black_box(String::new());
        let mut y = x.clone(); //~ redundant_clone
        black_box(&mut x);
        y = black_box(String::new());
        black_box(y);
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            let y = black_box(String::new());
            x = y.clone(); //~ redundant_clone
            black_box(&y);
        }
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            let y = black_box(String::new());
            x = y.clone(); //~ redundant_clone
            black_box(&y);
        }
        black_box(&x);
    }
    {
        let mut x = black_box(String::new());
        for _ in 0..10 {
            let y = x.clone(); //~ redundant_clone
            black_box(y);
            x = black_box(String::new());
        }
    }
    {
        let x = black_box(String::new());
        let y = if black_box(true) {
            x.clone() //~ redundant_clone
        } else {
            black_box(0);
            x.clone() //~ redundant_clone
        };
        black_box(y);
    }
    {
        let mut x = black_box(String::new());
        let mut y = black_box(String::new());
        for _ in 0..10 {
            y = x.clone(); //~ redundant_clone
            x = black_box(String::new());
        }
        black_box(&x);
    }
    {
        let mut x = black_box(String::new());
        let mut y = x.clone(); //~ redundant_clone
        for _ in 0..10 {
            black_box(y);
            x = black_box(String::new());
            y = x.clone(); //~ redundant_clone
        }
    }
    {
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = if black_box(true) { &x } else { &y };

        black_box(x.clone());
        black_box(y.clone());
        black_box(z.clone());

        black_box(&z);
    }
    {
        let mut x = black_box(String::new());
        let mut y = x.clone(); //~ redundant_clone
        let x2 = x;
        let y2 = y;
        x = black_box(String::new());
        y = black_box(String::new());
        black_box((x2, x, y));
    }
    {
        let mut x = black_box(String::new());
        let mut y = x.clone();
        let x2 = x;
        let y2 = y;
        x = black_box(String::new());
        y = black_box(String::new());
        black_box((x2, x, y, &y2));
    }
    {
        let x = black_box(Rc::new(String::new()));
        black_box(x.clone()); //~ redundant_clone
    }
    {
        let x = black_box(Arc::new(String::new()));
        black_box(x.clone()); //~ redundant_clone
    }
    {
        // Leak an `Rc` via `ManuallyDrop`
        let x = black_box(ManuallyDrop::new(Rc::new(String::new())));
        let _ = x.clone();
        let raw = Rc::into_raw(ManuallyDrop::into_inner(x));
        unsafe {
            let _ = Rc::from_raw(raw);
            let _ = Rc::from_raw(raw);
        }
    }
    {
        // Leak an `Arc` via `ManuallyDrop`
        let x = black_box(ManuallyDrop::new(Arc::new(String::new())));
        let _ = x.clone();
        let raw = Arc::into_raw(ManuallyDrop::into_inner(x));
        unsafe {
            let _ = Arc::from_raw(raw);
            let _ = Arc::from_raw(raw);
        }
    }
    {
        // Don't lint trivial clones
        let x = black_box(Range { start: 0, end: 0 });
        black_box(x.clone());
    }
    {
        let x = 5;
        #[allow(clippy::clone_on_copy)]
        black_box(x.clone());
    }
    {
        let x = black_box(String::new());
        #[allow(clippy::redundant_clone)]
        black_box(x.clone());
        #[expect(clippy::redundant_clone)]
        black_box(x.clone());
    }
    {
        let mut x = black_box(<(String, String, String, String, String)>::default());
        for _ in black_box(0..10) {
            x.4 = x.3;
            x.3 = x.2;
            x.2 = x.1;
            x.1 = x.0.clone(); //~ redundant_clone
        }
        black_box(x.0);
    }
    {
        let mut x = black_box(<(String, String, String, String, String)>::default());
        for _ in black_box(0..10) {
            x.4 = x.3;
            x.3 = x.2;
            x.2 = x.1;
            x.1 = x.0.clone();
        }
        black_box((x.4, x.3));
    }
    {
        let x = black_box(String::new());
        let mut y = black_box(String::new());
        let mut z = black_box(String::new());
        let (y, z) = if black_box(true) {
            y = x.clone(); //~ redundant_clone
            (&y, &z)
        } else {
            z = x.clone(); //~ redundant_clone
            (&y, &z)
        };
        black_box((y, z));
    }
    {
        let x = black_box(String::new());
        let mut y = black_box(String::new());
        let mut z = black_box(String::new());
        let (x, y) = if black_box(true) {
            y = x.clone();
            (&x, &y)
        } else {
            z = x.clone(); //~ redundant_clone
            (&x, &y)
        };
        black_box((x, y));
    }
    {
        let x = black_box(String::new());
        let mut y = black_box(String::new());
        let mut z = black_box(String::new());
        let (x, y, z) = if black_box(true) {
            y = x.clone();
            (&x, &y, &z)
        } else {
            z = x.clone();
            (&x, &y, &z)
        };
        black_box((x, y, z));
    }
    {
        let mut x = black_box((String::new(), 0));
        let y = x.clone(); //~ redundant_clone
        x.1 = 5;
        black_box(&y);
    }
    {
        let x = black_box((String::new(), 0));
        let mut y = x.clone(); //~ redundant_clone
        y.1 = 5;
        black_box(&x);
    }
    {
        let mut x = black_box((String::new(), String::new(), String::new()));
        black_box(&mut x);
        let y = if black_box(true) {
            x.0.clone()
        } else {
            black_box(String::new())
        };
        black_box((&x.0, &y));
    }
    {
        let mut x = black_box((String::new(), String::new(), String::new()));
        black_box(&mut x);
        let y = if black_box(true) {
            x.1.clone()
        } else {
            black_box(String::new())
        };
        black_box((&x.1, &y));
    }
    {
        let mut x = black_box((String::new(), String::new(), String::new()));
        black_box(&mut x);
        let y = if black_box(true) {
            x.2.clone()
        } else {
            black_box(String::new())
        };
        black_box((&x.2, &y));
    }
    {
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = (if black_box(true) { &x } else { &y }).clone();
        black_box((x, z));
    }
    {
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = (if black_box(true) { &x } else { &y }).clone();
        black_box((y, z));
    }
    {
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = {
            let x = (&x, &y);
            (
                x.0.clone(),
                x.1.clone(), //~ redundant_clone
            )
        };
        black_box((z.0, &z.1, &x, &y));
    }
    {
        struct X<'a> {
            x: &'a String,
            y: &'a String,
        }
        struct Y {
            x: String,
            y: String,
        }
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = {
            let x = &x;
            let y = &y;
            let z = X { x, y };
            let x = (z.x, z.y, x, y).0.clone();
            (x, z.y.clone()) //~ redundant_clone
        };
        let a = Y { x, y: z.1 };
        black_box((a.x, &z.0));
        black_box(y);
    }
    {
        let x = black_box((String::new(), String::new()));
        let y = (x.0.clone(), black_box(String::new()));
        black_box((&x, &y));
    }
    {
        let mut x = black_box(String::new());
        let y = black_box(&raw mut x);
        let z = x.clone();
        unsafe {
            *y = black_box(String::new());
        }
        black_box((&x, &z));
    }
    {
        let x = black_box(String::new());
        let y = black_box(&raw const x);
        let mut z = x.clone();
        black_box(&mut z);
        unsafe {
            black_box((&*y, &z));
        }
    }
    {
        let x = black_box((String::new(), String::new()));
        let y = black_box(&raw const x.0);
        let mut z = x.clone();
        black_box(&mut z);
        unsafe {
            black_box((&*y, &z));
        }
    }
    {
        let x = black_box((String::new(), String::new()));
        let y = black_box(&raw const x.0);
        let mut z = x.1.clone(); //~ redundant_clone
        black_box(&mut z);
        unsafe {
            black_box((&*y, &z));
        }
    }
    {
        let x = black_box(String::new());
        let y = black_box(String::new());
        let z = {
            let mut x = &x;
            unsafe { *black_box(&raw mut x) = &y }
            x.clone()
        };
        black_box((y, z));
    }
    {
        #[derive(Default, Clone)]
        struct X(String);
        struct Y(X);
        impl Drop for Y {
            fn drop(&mut self) {}
        }

        let x = black_box(Y(X(String::new())));
        black_box(x.0.clone()); //~ redundant_clone
    }
    {
        #[derive(Default, Clone)]
        struct X(String);
        struct Y(X);
        impl Drop for Y {
            fn drop(&mut self) {}
        }

        let x = black_box(Y(X(String::new())));
        black_box(x.0.clone());
        black_box(&x.0);
    }
    {
        #[derive(Clone)]
        struct X(String);
        struct Y(X);
        impl Drop for Y {
            fn drop(&mut self) {}
        }

        let x = black_box(Y(X(String::new())));
        black_box(x.0.clone());
    }
    {
        let x = black_box(String::new());
        let _ = black_box(x.to_string()); //~ redundant_clone
    }
    {
        let x = black_box(String::new());
        let _ = black_box(x.to_owned()); //~ redundant_clone
    }
    {
        let x = black_box(PathBuf::new());
        let _ = black_box(x.to_owned()); //~ redundant_clone
    }
    {
        struct X(String);
        impl ToOwned for X {
            type Owned = i32;
            fn to_owned(&self) -> Self::Owned {
                1
            }
        }
        impl Borrow<X> for i32 {
            fn borrow(&self) -> &X {
                panic!();
            }
        }

        let x = black_box(X(String::new()));
        let _ = black_box(x.to_owned());
    }
}
