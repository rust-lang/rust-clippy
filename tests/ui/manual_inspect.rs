#![warn(clippy::manual_inspect)]
#![allow(
    clippy::no_effect,
    clippy::op_ref,
    clippy::explicit_auto_deref,
    clippy::needless_borrow
)]

fn main() {
    let _ = Some(0).map(|x| {
        println!("{}", x);
        x
    });

    let _ = Some(0).map(|x| {
        println!("{x}");
        x
    });

    let _ = Some(0).map(|x| {
        println!("{}", x * 5 + 1);
        x
    });

    let _ = Some(0).map(|x| {
        if x == 0 {
            panic!();
        }
        x
    });

    let _ = Some(0).map(|x| {
        if &x == &0 {
            let _y = x;
            panic!();
        }
        x
    });

    let _ = Some(0).map(|x| {
        let y = x + 1;
        if y > 5 {
            return y;
        }
        x
    });

    {
        #[derive(PartialEq)]
        struct Foo(i32);

        let _ = Some(Foo(0)).map(|x| {
            if x == Foo(0) {
                panic!();
            }
            x
        });

        let _ = Some(Foo(0)).map(|x| {
            if &x == &Foo(0) {
                let _y = x;
                panic!();
            }
            x
        });
    }

    {
        macro_rules! maybe_ret {
            ($e:expr) => {
                if $e == 0 {
                    return $e;
                }
            };
        }

        let _ = Some(0).map(|x| {
            maybe_ret!(x);
            x
        });
    }

    let _ = Some((String::new(), 0u32)).map(|x| {
        if x.1 == 0 {
            let _x = x.1;
            panic!();
        }
        x
    });

    let _ = Some((String::new(), 0u32)).map(|x| {
        if x.1 == 0 {
            let _x = x.0;
            panic!();
        }
        x
    });

    let _ = Some(String::new()).map(|x| {
        if x.is_empty() {
            let _ = || {
                let _x = x;
            };
            panic!();
        }
        x
    });

    let _ = Some(String::new()).map(|x| {
        if x.is_empty() {
            let _ = || {
                let _x = &x;
            };
            return x;
        }
        println!("test");
        x
    });

    let _ = Some(0).map(|x| {
        if x == 0 {
            let _ = || {
                let _x = x;
            };
            panic!();
        }
        x
    });

    {
        use core::cell::Cell;
        #[derive(Debug)]
        struct Cell2(core::cell::Cell<u32>);

        let _ = Some(Cell2(Cell::new(0u32))).map(|x| {
            x.0.set(1);
            x
        });

        let _ = Some(Cell2(Cell::new(0u32))).map(|x| {
            let y = &x;
            if x.0.get() == 0 {
                y.0.set(1)
            } else {
                println!("{x:?}");
            }
            x
        });
    }

    let _: Result<_, ()> = Ok(0).map(|x| {
        println!("{}", x);
        x
    });

    let _: Result<(), _> = Err(0).map_err(|x| {
        println!("{}", x);
        x
    });

    let _ = [0]
        .into_iter()
        .map(|x| {
            println!("{}", x);
            x
        })
        .count();

    {
        struct S<T>(T);
        impl<T> S<T> {
            fn map<U>(self, f: impl FnOnce(T) -> U) -> S<U> {
                S(f(self.0))
            }

            fn map_err<U>(self, f: impl FnOnce(T) -> U) -> S<U> {
                S(f(self.0))
            }
        }

        let _ = S(0).map(|x| {
            println!("{}", x);
            x
        });

        let _ = S(0).map_err(|x| {
            println!("{}", x);
            x
        });
    }
}

fn issue_13185() {
    struct T(u32);

    impl T {
        fn do_immut(&self) {
            println!("meow~");
        }

        fn do_mut(&mut self) {
            self.0 += 514;
        }
    }

    _ = Some(T(114)).as_mut().map(|t| {
        t.0 + 514;
        t
    });

    _ = Some(T(114)).as_mut().map(|t| {
        t.0 = 514;
        t
    });

    _ = Some(T(114)).as_mut().map(|t| {
        t.0 += 514;
        t
    });

    // FIXME: It's better to lint this case
    _ = Some(T(114)).as_mut().map(|t| {
        let indirect = t;
        indirect.0 + 514;
        indirect
    });

    _ = Some(T(114)).as_mut().map(|t| {
        let indirect = t;
        indirect.0 += 514;
        indirect
    });

    _ = Some(T(114)).as_mut().map(|t| {
        t.do_mut();
        t
    });

    _ = Some(T(114)).as_mut().map(|t| {
        t.do_immut();
        t
    });

    _ = Some(T(114)).as_mut().map(|t| {
        T::do_mut(t);
        t
    });

    _ = Some(T(114)).as_mut().map(|t| {
        T::do_immut(t);
        t
    });

    // FIXME: It's better to lint this case
    _ = Some(T(114)).as_mut().map(|t| {
        let indirect = t;
        indirect.do_immut();
        indirect
    });

    // FIXME: It's better to lint this case
    _ = Some(T(114)).as_mut().map(|t| {
        (&*t).do_immut();
        t
    });

    // Nested fields access
    struct N(T);

    _ = Some(N(T(114))).as_mut().map(|t| {
        t.0.do_mut();
        t
    });

    _ = Some(N(T(114))).as_mut().map(|t| {
        t.0.do_immut();
        t
    });

    _ = Some(N(T(114))).as_mut().map(|t| {
        T::do_mut(&mut t.0);
        t
    });

    _ = Some(N(T(114))).as_mut().map(|t| {
        T::do_immut(&t.0);
        t
    });
}
