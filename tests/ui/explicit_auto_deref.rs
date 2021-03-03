// run-rustfix

#![allow(
    unused_imports,
    dead_code,
    clippy::borrowed_box,
    clippy::deref_addrof,
    clippy::useless_vec
)]
#![warn(clippy::explicit_auto_deref)]

use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

fn main() {
    let _: &str = &*String::new();
    let _: &Path = &*PathBuf::new();
    let _: &[_] = vec![0u32, 1u32, 2u32].deref();

    fn f2<T>(_: &[T]) {}
    f2(vec![0u32].deref());

    fn f3() -> &'static str {
        static S: String = String::new();
        &*S
    }

    fn f4<T1, T2>(_: &(T1, T2)) {}
    f4(Box::new((0u32, 0i32)).deref());

    let f5 = |_: &str| {};
    f5(&*String::new());

    fn f6(f: impl Fn(&str)) {
        f(&*String::new());
    }

    fn f7(f: &dyn Fn(&str)) {
        f(&*String::new());
    }

    let _: &Box<_> = &*Box::new(Box::new(0));
    let _: Box<u32> = *Box::new(Box::new(0));

    fn f8<T: ?Sized>(_: &T) {}
    f8(&*String::new());

    struct S1<T>(T);
    impl<T: Fn(&str)> S1<T> {
        fn f(&self, f: impl Fn(&str)) {
            f(&*String::new());
            (self.0)(&*String::new());
        }
    }

    fn f9<T>(f: &mut T)
    where
        T: Iterator,
        <T as Iterator>::Item: Fn(&str),
    {
        f.next().unwrap()(&*String::new())
    }

    struct S2<'a> {
        x: &'a str,
    }
    let _ = S2 { x: &*String::new() };

    struct S3<'a>(&'a str);
    let _ = S3(&*String::new());

    macro_rules! m1 {
        ($e:expr) => {{
            fn f(_: &str) {}
            f($e);
        }};
    }
    m1!(&*String::new());

    macro_rules! m2 {
        ($e:expr) => {
            &$e
        };
    }
    let _: &str = &**m2!(String::new());
}
