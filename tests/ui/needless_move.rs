//! To properly check that the `needless_move` lint is complete, go to the
//! `.fixed` file of this test and check that the code fails to compile if
//! any of the `move`s are removed.

#![warn(clippy::needless_move)]
#![allow(unused)]
#![allow(ungated_async_fn_track_caller)]
#![allow(clippy::useless_format)]
#![allow(clippy::let_and_return)]
#![allow(clippy::no_effect)]
#![allow(clippy::box_collection)]
#![allow(clippy::boxed_local)]
#![allow(clippy::disallowed_names)]
#![allow(clippy::manual_async_fn)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::unused_unit)]
#![feature(async_closure)]
#![feature(raw_ref_op)]

#[derive(Copy, Clone)]
struct Copy;

struct NonCopy;

struct Composite {
    copy: Copy,
    non_copy: NonCopy,
}

impl Composite {
    fn new() -> Self {
        Self {
            copy: Copy,
            non_copy: NonCopy,
        }
    }
}

fn with_owned<T>(_: T) {}
fn with_ref<T>(_: &T) {}
fn with_ref_mut<T>(_: &mut T) {}
fn assert_static<T: 'static>(v: T) -> T {
    v
}

fn main() {
    // doesn't trigger on non-move closures or async blocks
    let a = NonCopy;
    let b = Copy;
    let closure = || {
        with_owned(a);
        with_owned(b);
    };

    let a = NonCopy;
    let b = Copy;
    let fut = async {
        with_owned(a);
        with_owned(b);
    };

    // doesn't trigger on async fns

    // (an async fn is a fn whose body turns into an `async move {}` block, where the `move` kw has
    // DUMMY_SP as the Span). It shouldn't trigger the lint.
    async fn f() {}

    // triggers on move closures and async blocks which do not capture anything
    let closure = assert_static(move || {});
    let fut = assert_static(async move {});

    // owned + NonCopy
    let a = NonCopy;
    let closure = assert_static(move || {
        with_owned(a);
    });

    // owned + Copy
    let a = Copy;
    let closure = assert_static(move || {
        with_owned(a);
    });

    // ref + NonCopy
    let a = NonCopy;
    let closure = assert_static(move || {
        with_ref(&a);
    });

    // ref + Copy
    let a = Copy;
    let closure = assert_static(move || {
        with_ref(&a);
    });

    // ref mut + NonCopy
    let mut a = NonCopy;
    let closure = assert_static(move || {
        with_ref_mut(&mut a);
    });

    // ref mut + Copy
    let mut a = Copy;
    let closure = assert_static(move || {
        with_ref_mut(&mut a);
    });

    // with async

    // doesn't trigger if not capturing with `move`
    let a = NonCopy;
    let b = Copy;
    let fut = async {
        with_owned(a);
        with_owned(b);
    };

    // owned + non-copy
    let a = NonCopy;
    let fut = assert_static(async move {
        with_owned(a);
    });

    // owned + copy
    let a = Copy;
    let fut = assert_static(async move {
        with_owned(a);
    });

    // ref + non-copy
    let a = NonCopy;
    let fut = assert_static(async move {
        with_ref(&a);
    });

    // ref + copy
    let a = Copy;
    let fut = assert_static(async move {
        with_ref(&a);
    });

    // ref mut + non-copy
    let mut a = NonCopy;
    let fut = assert_static(async move {
        with_ref_mut(&mut a);
    });

    // ref mut + copy
    let mut a = Copy;
    let fut = assert_static(async move {
        with_ref_mut(&mut a);
    });

    // triggers on ref + owned combinations
    // ref + owned + non copy
    let a = NonCopy;
    let closure = assert_static(move || {
        with_ref(&a);
        with_owned(a);
    });

    // ref + owned + copy
    let a = Copy;
    let closure = assert_static(move || {
        with_ref(&a);
        with_owned(a);
    });

    // ref mut + owned + non copy
    let mut a = NonCopy;
    let closure = assert_static(move || {
        with_ref_mut(&mut a);
        with_owned(a);
    });

    // ref mut + owned + copy
    let mut a = Copy;
    let closure = assert_static(move || {
        with_ref_mut(&mut a);
        with_owned(a);
    });

    // ref + owned + non copy + other owned capture in between
    let a = NonCopy;
    let b = NonCopy;
    let closure = assert_static(move || {
        with_ref(&a);
        with_owned(b);
        with_owned(a);
    });

    // ref + owned + copy + other owned capture in between
    let a = Copy;
    let b = NonCopy;
    let closure = assert_static(move || {
        with_ref(&a);
        with_owned(b);
        with_owned(a);
    });

    // with composite structures + disjoint captures

    // owned
    let a = Composite::new();
    let closure = assert_static(move || {
        with_owned(a);
    });

    // ref
    let a = Composite::new();
    let closure = assert_static(move || {
        with_ref(&a);
    });

    // ref mut
    let mut a = Composite::new();
    let closure = assert_static(move || {
        with_ref_mut(&mut a);
    });

    // capturing only the copy part
    // owned
    let a = Composite::new();
    let closure = assert_static(move || {
        with_owned(a.copy);
    });

    // ref
    let a = Composite::new();
    let closure = assert_static(move || {
        with_ref(&a.copy);
    });

    // ref mut
    let mut a = Composite::new();
    let closure = assert_static(move || {
        with_ref_mut(&mut a.copy);
    });

    // capturing only the non-copy part
    // owned
    let a = Composite::new();
    let closure = assert_static(move || {
        with_owned(a.non_copy);
    });

    // ref
    let a = Composite::new();
    let closure = assert_static(move || {
        with_ref(&a.non_copy);
    });

    // ref mut
    let mut a = Composite::new();
    let closure = assert_static(move || {
        with_ref_mut(&mut a.non_copy);
    });

    // capturing both parts
    // owned
    let a = Composite::new();
    let closure = assert_static(move || {
        with_owned(a.copy);
        with_owned(a.non_copy);
    });

    // ref
    let a = Composite::new();
    let closure = assert_static(move || {
        with_ref(&a.copy);
        with_ref(&a.non_copy);
    });

    // ref mut
    let mut a = Composite::new();
    let closure = assert_static(move || {
        with_ref_mut(&mut a.copy);
        with_ref_mut(&mut a.non_copy);
    });

    // correctly handles direct mutations of variables
    // copy
    let mut a = Copy;
    let closure = assert_static(move || {
        a = Copy;
    });

    // non-copy
    let mut a = NonCopy;
    let closure = assert_static(move || {
        a = NonCopy;
    });

    // composite -> copy
    let mut a = Composite::new();
    let closure = assert_static(move || {
        a.copy = Copy;
    });

    // composite -> non-copy
    let mut a = Composite::new();
    let closure = assert_static(move || {
        a.non_copy = NonCopy;
    });

    // copy + owned consume
    let mut a = Copy;
    let closure = assert_static(move || {
        a = Copy;
        with_owned(a);
    });

    // non-copy + owned consume
    let mut a = NonCopy;
    let closure = assert_static(move || {
        a = NonCopy;
        with_owned(a);
    });

    // composite -> copy + owned consume
    let mut a = Composite::new();
    let closure = assert_static(move || {
        a.copy = Copy;
        with_owned(a);
    });

    // composite -> non-copy + owned consume
    let mut a = Composite::new();
    let closure = assert_static(move || {
        a.non_copy = NonCopy;
        with_owned(a);
    });

    // async blocks
    // copy
    let mut a = Copy;
    let fut = assert_static(async move {
        a = Copy;
    });

    // non-copy
    let mut a = NonCopy;
    let fut = assert_static(async move {
        a = NonCopy;
    });

    // composite -> copy
    let mut a = Composite::new();
    let fut = assert_static(async move {
        a.copy = Copy;
    });

    // composite -> non-copy
    let mut a = Composite::new();
    let fut = assert_static(async move {
        a.non_copy = NonCopy;
    });

    // copy + owned consume
    let mut a = Copy;
    let fut = assert_static(async move {
        a = Copy;
        with_owned(a);
    });

    // non-copy + owned consume
    let mut a = NonCopy;
    let fut = assert_static(async move {
        a = NonCopy;
        with_owned(a);
    });

    // composite -> copy + owned consume
    let mut a = Composite::new();
    let fut = assert_static(async move {
        a.copy = Copy;
        with_owned(a);
    });

    // composite -> non-copy + owned consume
    let mut a = Composite::new();
    let fut = assert_static(async move {
        a.non_copy = NonCopy;
        with_owned(a);
    });

    let v = (String::new(), String::new());
    assert_static(move || {
        let _w = v.0;
        let _h = &v.1;
    });

    // below are a few tests from rustc's testsuite that use move closures,
    // which might uncover edge cases

    // rust/tests/ui/closures/2229_closure_analysis/migrations/no_migrations.rs

    fn _no_migrations() {
        // Set of test cases that don't need migrations

        #![deny(rust_2021_incompatible_closure_captures)]

        // Copy types as copied by the closure instead of being moved into the closure
        // Therefore their drop order isn't tied to the closure and won't be requiring any
        // migrations.
        fn test1_only_copy_types() {
            let t = (0i32, 0i32);

            let c = || {
                let _t = t.0;
            };

            c();
        }

        // Same as test1 but using a move closure
        fn test2_only_copy_types_move_closure() {
            let t = (0i32, 0i32);

            let c = move || {
                println!("{}", t.0);
            };

            c();
        }

        // Don't need to migrate if captured by ref
        fn test3_only_copy_types_move_closure() {
            let t = (String::new(), String::new());

            let c = || {
                println!("{}", t.0);
            };

            c();
        }

        // Test migration analysis in case of Insignificant Drop + Non Drop aggregates.
        // Note in this test the closure captures a non Drop type and therefore the variable
        // is only captured by ref.
        fn test4_insignificant_drop_non_drop_aggregate() {
            let t = (String::new(), 0i32);

            let c = || {
                let _t = t.1;
            };

            c();
        }

        struct Foo(i32);
        impl Drop for Foo {
            fn drop(&mut self) {
                println!("{:?} dropped", self.0);
            }
        }

        // Test migration analysis in case of Significant Drop + Non Drop aggregates.
        // Note in this test the closure captures a non Drop type and therefore the variable
        // is only captured by ref.
        fn test5_significant_drop_non_drop_aggregate() {
            let t = (Foo(0), 0i32);

            let c = || {
                let _t = t.1;
            };

            c();
        }

        fn main() {
            test1_only_copy_types();
            test2_only_copy_types_move_closure();
            test3_only_copy_types_move_closure();
            test4_insignificant_drop_non_drop_aggregate();
            test5_significant_drop_non_drop_aggregate();
        }
    }

    // rust/tests/ui/closures/2229_closure_analysis/run_pass/issue-88476.rs

    fn _issue_88476() {
        use std::rc::Rc;

        // Test that we restrict precision when moving not-`Copy` types, if any of the parent paths
        // implement `Drop`. This is to ensure that we don't move out of a type that implements Drop.
        pub fn test1() {
            struct Foo(Rc<i32>);

            impl Drop for Foo {
                fn drop(self: &mut Foo) {}
            }

            let f = Foo(Rc::new(1));
            let x = move || {
                println!("{:?}", f.0);
            };

            x();
        }

        // Test that we don't restrict precision when moving `Copy` types(i.e. when copying),
        // even if any of the parent paths implement `Drop`.
        pub fn test2() {
            struct Character {
                hp: u32,
                name: String,
            }

            impl Drop for Character {
                fn drop(&mut self) {}
            }

            let character = Character {
                hp: 100,
                name: format!("A"),
            };

            let c = move || println!("{}", character.hp);

            c();

            println!("{}", character.name);
        }

        fn main() {}
    }

    // rust/tests/ui/closures/2229_closure_analysis/preserve_field_drop_order2.rs

    fn _preserve_field_drop_order2() {
        #[derive(Debug)]
        struct Dropable(&'static str);

        impl Drop for Dropable {
            fn drop(&mut self) {
                println!("Dropping {}", self.0)
            }
        }

        #[derive(Debug)]
        struct A {
            x: Dropable,
            y: Dropable,
        }

        #[derive(Debug)]
        struct B {
            c: A,
            d: A,
        }

        #[derive(Debug)]
        struct R<'a> {
            c: &'a A,
            d: &'a A,
        }

        fn main() {
            let a = A {
                x: Dropable("x"),
                y: Dropable("y"),
            };

            let c = move || println!("{:?} {:?}", a.y, a.x);

            c();

            let b = B {
                c: A {
                    x: Dropable("b.c.x"),
                    y: Dropable("b.c.y"),
                },
                d: A {
                    x: Dropable("b.d.x"),
                    y: Dropable("b.d.y"),
                },
            };

            let d = move || println!("{:?} {:?} {:?} {:?}", b.d.y, b.d.x, b.c.y, b.c.x);

            d();

            let r = R {
                c: &A {
                    x: Dropable("r.c.x"),
                    y: Dropable("r.c.y"),
                },
                d: &A {
                    x: Dropable("r.d.x"),
                    y: Dropable("r.d.y"),
                },
            };

            let e = move || println!("{:?} {:?} {:?} {:?}", r.d.y, r.d.x, r.c.y, r.c.x);

            e();
        }
    }

    // rust/tests/ui/closures/issue-72408-nested-closures-exponential.rs

    fn _issue_72408_nested_closures_exponential() {

        /*
        // commented out because it takes forever to run with this

        // Closures include captured types twice in a type tree.
        //
        // Wrapping one closure with another leads to doubling
        // the amount of types in the type tree.
        //
        // This test ensures that rust can handle
        // deeply nested type trees with a lot
        // of duplicated subtrees.

        fn dup(f: impl Fn(i32) -> i32) -> impl Fn(i32) -> i32 {
            move |a| f(a * 2)
        }

        fn main() {
            let f = |a| a;

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            // Compiler dies around here if it tries
            // to walk the tree exhaustively.

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);
            let f = dup(f);

            println!("Type size was at least {}", f(1));
        }

        */
    }

    // rust/tests/ui/closures/issue-97607.rs

    fn _issue_97607() {
        #[allow(unused)]

        fn test<T, F, U>(f: F) -> Box<dyn Fn(T) -> U + 'static>
        where
            F: 'static + Fn(T) -> U,
            for<'a> U: 'a, // < This is the problematic line, see #97607
        {
            Box::new(move |t| f(t))
        }

        fn main() {}
    }

    // rust/tests/ui/closures/once-move-out-on-heap.rs

    fn _once_move_out_on_heap() {
        // Testing guarantees provided by once functions.

        use std::sync::Arc;

        fn foo<F: FnOnce()>(blk: F) {
            blk();
        }

        pub fn main() {
            let x = Arc::new(true);
            foo(move || {
                assert!(*x);
                drop(x);
            });
        }
    }

    // rust/tests/ui/closures/supertrait-hint-references-assoc-ty.rs

    fn _supertrait_hint_references_assoc_ty() {
        pub trait Fn0: Fn(i32) -> Self::Out {
            type Out;
        }

        impl<F: Fn(i32) -> ()> Fn0 for F {
            type Out = ();
        }

        pub fn closure_typer(_: impl Fn0) {}

        fn main() {
            closure_typer(move |x| {
                let _: i64 = x.into();
            });
        }
    }

    // rust/tests/ui/unboxed-closures/issue-18652.rs

    fn _issue_18652() {
        // Tests multiple free variables being passed by value into an unboxed
        // once closure as an optimization by codegen.  This used to hit an
        // incorrect assert.

        fn main() {
            let x = 2u8;
            let y = 3u8;
            assert_eq!((move || x + y)(), 5);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-all-traits.rs

    fn _unboxed_closures_all_traits() {
        fn a<F: Fn(isize, isize) -> isize>(f: F) -> isize {
            f(1, 2)
        }

        fn b<F: FnMut(isize, isize) -> isize>(mut f: F) -> isize {
            f(3, 4)
        }

        fn c<F: FnOnce(isize, isize) -> isize>(f: F) -> isize {
            f(5, 6)
        }

        fn main() {
            let z: isize = 7;
            assert_eq!(a(move |x: isize, y| x + y + z), 10);
            assert_eq!(b(move |x: isize, y| x + y + z), 14);
            assert_eq!(c(move |x: isize, y| x + y + z), 18);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-boxed.rs

    fn _unboxed_closures_boxed() {
        use std::ops::FnMut;

        fn make_adder(x: i32) -> Box<dyn FnMut(i32) -> i32 + 'static> {
            Box::new(move |y: i32| -> i32 { x + y }) as Box<dyn FnMut(i32) -> i32 + 'static>
        }

        pub fn main() {
            let mut adder = make_adder(3);
            let z = adder(2);
            println!("{}", z);
            assert_eq!(z, 5);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-call-sugar-object-autoderef.rs

    fn _unboxed_closures_call_sugar_object_autoderef() {
        // Test that the call operator autoderefs when calling to an object type.

        use std::ops::FnMut;

        fn make_adder(x: isize) -> Box<dyn FnMut(isize) -> isize + 'static> {
            Box::new(move |y| x + y)
        }

        pub fn main() {
            let mut adder = make_adder(3);
            let z = adder(2);
            println!("{}", z);
            assert_eq!(z, 5);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-call-sugar-object.rs

    fn _unboxed_closures_call_sugar_object() {
        use std::ops::FnMut;

        fn make_adder(x: isize) -> Box<dyn FnMut(isize) -> isize + 'static> {
            Box::new(move |y| x + y)
        }

        pub fn main() {
            let mut adder = make_adder(3);
            let z = (*adder)(2);
            println!("{}", z);
            assert_eq!(z, 5);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-counter-not-moved.rs

    fn _unboxed_closures_counter_not_moved() {
        // Test that we mutate a counter on the stack only when we expect to.

        fn call<F>(f: F)
        where
            F: FnOnce(),
        {
            f();
        }

        fn main() {
            let y = vec![format!("Hello"), format!("World")];
            let mut counter = 22_u32;

            call(|| {
                // Move `y`, but do not move `counter`, even though it is read
                // by value (note that it is also mutated).
                for item in y {
                    let v = counter;
                    counter += v;
                }
            });
            assert_eq!(counter, 88);

            call(move || {
                // this mutates a moved copy, and hence doesn't affect original
                counter += 1;
            });
            assert_eq!(counter, 88);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-drop.rs

    fn _unboxed_closures_drop() {
        #![allow(path_statements)]
        #![allow(dead_code)]
        // A battery of tests to ensure destructors of unboxed closure environments
        // run at the right times.

        static mut DROP_COUNT: usize = 0;

        fn drop_count() -> usize {
            unsafe { DROP_COUNT }
        }

        struct Droppable {
            x: isize,
        }

        impl Droppable {
            fn new() -> Droppable {
                Droppable { x: 1 }
            }
        }

        impl Drop for Droppable {
            fn drop(&mut self) {
                unsafe { DROP_COUNT += 1 }
            }
        }

        fn a<F: Fn(isize, isize) -> isize>(f: F) -> isize {
            f(1, 2)
        }

        fn b<F: FnMut(isize, isize) -> isize>(mut f: F) -> isize {
            f(3, 4)
        }

        fn c<F: FnOnce(isize, isize) -> isize>(f: F) -> isize {
            f(5, 6)
        }

        fn test_fn() {
            {
                a(move |a: isize, b| a + b);
            }
            assert_eq!(drop_count(), 0);

            {
                let z = &Droppable::new();
                a(move |a: isize, b| {
                    z;
                    a + b
                });
                assert_eq!(drop_count(), 0);
            }
            assert_eq!(drop_count(), 1);

            {
                let z = &Droppable::new();
                let zz = &Droppable::new();
                a(move |a: isize, b| {
                    z;
                    zz;
                    a + b
                });
                assert_eq!(drop_count(), 1);
            }
            assert_eq!(drop_count(), 3);
        }

        fn test_fn_mut() {
            {
                b(move |a: isize, b| a + b);
            }
            assert_eq!(drop_count(), 3);

            {
                let z = &Droppable::new();
                b(move |a: isize, b| {
                    z;
                    a + b
                });
                assert_eq!(drop_count(), 3);
            }
            assert_eq!(drop_count(), 4);

            {
                let z = &Droppable::new();
                let zz = &Droppable::new();
                b(move |a: isize, b| {
                    z;
                    zz;
                    a + b
                });
                assert_eq!(drop_count(), 4);
            }
            assert_eq!(drop_count(), 6);
        }

        fn test_fn_once() {
            {
                c(move |a: isize, b| a + b);
            }
            assert_eq!(drop_count(), 6);

            {
                let z = Droppable::new();
                c(move |a: isize, b| {
                    z;
                    a + b
                });
                assert_eq!(drop_count(), 7);
            }
            assert_eq!(drop_count(), 7);

            {
                let z = Droppable::new();
                let zz = Droppable::new();
                c(move |a: isize, b| {
                    z;
                    zz;
                    a + b
                });
                assert_eq!(drop_count(), 9);
            }
            assert_eq!(drop_count(), 9);
        }

        fn main() {
            test_fn();
            test_fn_mut();
            test_fn_once();
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-infer-fnmut-move.rs

    fn _unboxed_closures_infer_fnmut_move() {
        // Test that we are able to infer a suitable kind for this `move`
        // closure that is just called (`FnMut`).

        fn main() {
            let mut counter = 0;

            let v = {
                let mut tick = move || {
                    counter += 1;
                    counter
                };
                tick();
                tick()
            };

            assert_eq!(counter, 0);
            assert_eq!(v, 2);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-infer-fnonce-move.rs

    fn _unboxed_closures_infer_fnonce_move() {
        // Test that we are able to infer a suitable kind for this `move`
        // closure that is just called (`FnOnce`).

        use std::mem;

        struct DropMe<'a>(&'a mut i32);

        impl<'a> Drop for DropMe<'a> {
            fn drop(&mut self) {
                *self.0 += 1;
            }
        }

        fn main() {
            let mut counter = 0;

            {
                let drop_me = DropMe(&mut counter);
                let tick = move || mem::drop(drop_me);
                tick();
            }

            assert_eq!(counter, 1);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-monomorphization.rs

    fn _unboxed_closures_monomorphization() {
        // Test that unboxed closures in contexts with free type parameters
        // monomorphize correctly (issue #16791)

        fn main() {
            fn bar<'a, T: Clone + 'a>(t: T) -> Box<dyn FnMut() -> T + 'a> {
                Box::new(move || t.clone())
            }

            let mut f = bar(42_u32);
            assert_eq!(f(), 42);

            let mut f = bar("forty-two");
            assert_eq!(f(), "forty-two");

            let x = 42_u32;
            let mut f = bar(&x);
            assert_eq!(f(), &x);

            #[derive(Clone, Copy, Debug, PartialEq)]
            struct Foo(usize, &'static str);

            let x = Foo(42, "forty-two");
            let mut f = bar(x);
            assert_eq!(f(), x);
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-move-mutable.rs

    fn _unboxed_closures_move_mutable() {
        // pretty-expanded FIXME #23616

        #![deny(unused_mut)]
        #![allow(unused_must_use)]

        // Test that mutating a mutable upvar in a capture-by-value unboxed
        // closure does not ice (issue #18238) and marks the upvar as used
        // mutably so we do not get a spurious warning about it not needing to
        // be declared mutable (issue #18336 and #18769)

        fn set(x: &mut usize) {
            *x = 42;
        }

        fn main() {
            {
                let mut x = 0_usize;
                move || x += 1;
            }
            {
                let mut x = 0_usize;
                move || x += 1;
            }
            {
                let mut x = 0_usize;
                move || set(&mut x);
            }
            {
                let mut x = 0_usize;
                move || set(&mut x);
            }
        }
    }

    // rust/tests/ui/unboxed-closures/unboxed-closures-single-word-env.rs

    fn _unboxed_closures_single_word_env() {
        // Ensures that single-word environments work right in unboxed closures.
        // These take a different path in codegen.

        fn a<F: Fn(isize, isize) -> isize>(f: F) -> isize {
            f(1, 2)
        }

        fn b<F: FnMut(isize, isize) -> isize>(mut f: F) -> isize {
            f(3, 4)
        }

        fn c<F: FnOnce(isize, isize) -> isize>(f: F) -> isize {
            f(5, 6)
        }

        fn main() {
            let z = 10;
            assert_eq!(a(move |x: isize, y| x + y + z), 13);
            assert_eq!(b(move |x: isize, y| x + y + z), 17);
            assert_eq!(c(move |x: isize, y| x + y + z), 21);
        }
    }

    // rust/tests/ui/functions-closures/clone-closure.rs

    fn _clone_closure() {
        // Check that closures implement `Clone`.

        #[derive(Clone)]
        struct S(i32);

        fn main() {
            let mut a = S(5);
            let mut hello = move || {
                a.0 += 1;
                println!("Hello {}", a.0);
                a.0
            };

            let mut hello2 = hello.clone();
            assert_eq!(6, hello2());
            assert_eq!(6, hello());
        }
    }

    // rust/tests/ui/functions-closures/closure-bounds-can-capture-chan.rs

    fn _closure_bounds_can_capture_chan() {
        // pretty-expanded FIXME #23616

        use std::sync::mpsc::channel;

        fn foo<F: FnOnce() + Send>(blk: F) {
            blk();
        }

        pub fn main() {
            let (tx, rx) = channel();
            foo(move || {
                tx.send(()).unwrap();
            });
            rx.recv().unwrap();
        }
    }

    // rust/tests/ui/functions-closures/nullable-pointer-opt-closures.rs

    fn _nullable_pointer_opt_closures() {
        use std::mem;

        pub fn main() {
            // By Ref Capture
            let a = 10i32;
            let b = Some(|| println!("{}", a));
            // When we capture by reference we can use any of the
            // captures as the discriminant since they're all
            // behind a pointer.
            assert_eq!(mem::size_of_val(&b), mem::size_of::<usize>());

            // By Value Capture
            let a = Box::new(12i32);
            let b = Some(move || println!("{}", a));
            // We captured `a` by value and since it's a `Box` we can use it
            // as the discriminant.
            assert_eq!(mem::size_of_val(&b), mem::size_of::<Box<i32>>());

            // By Value Capture - Transitive case
            let a = "Hello".to_string(); // String -> Vec -> Unique -> NonZero
            let b = Some(move || println!("{}", a));
            // We captured `a` by value and since down the chain it contains
            // a `NonZero` field, we can use it as the discriminant.
            assert_eq!(mem::size_of_val(&b), mem::size_of::<String>());

            // By Value - No Optimization
            let a = 14i32;
            let b = Some(move || println!("{}", a));
            // We captured `a` by value but we can't use it as the discriminant
            // thus we end up with an extra field for the discriminant
            assert_eq!(mem::size_of_val(&b), mem::size_of::<(i32, i32)>());
        }
    }

    // rust/tests/ui/moves/moves-based-on-type-capture-clause.rs

    fn _moves_based_on_type_capture_clause() {
        #![allow(unused_must_use)]
        // ignore-emscripten no threads support

        use std::thread;

        pub fn main() {
            let x = "Hello world!".to_string();
            thread::spawn(move || {
                println!("{}", x);
            })
            .join();
        }
    }

    // rust/tests/ui/borrowck/borrow-raw-address-of-mutability-ok.rs

    fn _borrow_raw_address_of_mutability_ok() {
        fn mutable_address_of() {
            let mut x = 0;
            let y = &raw mut x;
        }

        fn mutable_address_of_closure() {
            let mut x = 0;
            let mut f = || {
                let y = &raw mut x;
            };
            f();
        }

        fn const_address_of_closure() {
            let x = 0;
            let f = || {
                let y = &raw const x;
            };
            f();
        }

        fn make_fn<F: Fn()>(f: F) -> F {
            f
        }

        fn const_address_of_fn_closure() {
            let x = 0;
            let f = make_fn(|| {
                let y = &raw const x;
            });
            f();
        }

        fn const_address_of_fn_closure_move() {
            let x = 0;
            let f = make_fn(move || {
                let y = &raw const x;
            });
            f();
        }

        fn main() {}
    }

    // rust/tests/ui/borrowck/kindck-implicit-close-over-mut-var.rs

    fn _kindck_implicit_close_over_mut_var() {
        #![allow(unused_must_use)]
        #![allow(dead_code)]
        use std::thread;

        fn user(_i: isize) {}

        fn foo() {
            // Here, i is *copied* into the proc (heap closure).
            // Requires allocation.  The proc's copy is not mutable.
            let mut i = 0;
            let t = thread::spawn(move || {
                user(i);
                println!("spawned {}", i)
            });
            i += 1;
            println!("original {}", i);
            t.join();
        }

        fn bar() {
            // Here, the original i has not been moved, only copied, so is still
            // mutable outside of the proc.
            let mut i = 0;
            while i < 10 {
                let t = thread::spawn(move || {
                    user(i);
                });
                i += 1;
                t.join();
            }
        }

        fn car() {
            // Here, i must be shadowed in the proc to be mutable.
            let mut i = 0;
            while i < 10 {
                let t = thread::spawn(move || {
                    let mut i = i;
                    i += 1;
                    user(i);
                });
                i += 1;
                t.join();
            }
        }

        pub fn main() {}
    }

    // rust/tests/ui/async-await/track-caller/panic-track-caller.rs

    fn _panic_track_caller() {
        // needs-unwind
        // gate-test-async_fn_track_caller
        #![cfg_attr(afn, feature(async_fn_track_caller))]
        #![cfg_attr(cls, feature(closure_track_caller))]
        #![allow(unused)]

        use std::future::Future;
        use std::panic;
        use std::sync::{Arc, Mutex};
        use std::task::{Context, Poll, Wake};
        use std::thread::{self, Thread};

        /// A waker that wakes up the current thread when called.
        struct ThreadWaker(Thread);

        impl Wake for ThreadWaker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }

        /// Run a future to completion on the current thread.
        fn block_on<T>(fut: impl Future<Output = T>) -> T {
            // Pin the future so it can be polled.
            let mut fut = Box::pin(fut);

            // Create a new context to be passed to the future.
            let t = thread::current();
            let waker = Arc::new(ThreadWaker(t)).into();
            let mut cx = Context::from_waker(&waker);

            // Run the future to completion.
            loop {
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(res) => return res,
                    Poll::Pending => thread::park(),
                }
            }
        }

        async fn bar() {
            panic!()
        }

        async fn foo() {
            bar().await
        }

        #[track_caller]
        async fn bar_track_caller() {
            panic!()
        }

        async fn foo_track_caller() {
            bar_track_caller().await
        }

        struct Foo;

        impl Foo {
            #[track_caller]
            async fn bar_assoc() {
                panic!();
            }
        }

        async fn foo_assoc() {
            Foo::bar_assoc().await
        }

        // Since compilation is expected to fail for this fn when using
        // `nofeat`, we test that separately in `async-closure-gate.rs`
        #[cfg(cls)]
        async fn foo_closure() {
            let c = #[track_caller]
            async || {
                panic!();
            };
            c().await
        }

        // Since compilation is expected to fail for this fn when using
        // `nofeat`, we test that separately in `async-block.rs`
        #[cfg(cls)]
        async fn foo_block() {
            let a = #[track_caller]
            async {
                panic!();
            };
            a.await
        }

        fn panicked_at(f: impl FnOnce() + panic::UnwindSafe) -> u32 {
            let loc = Arc::new(Mutex::new(None));

            let hook = panic::take_hook();
            {
                let loc = loc.clone();
                panic::set_hook(Box::new(move |info| {
                    *loc.lock().unwrap() = info.location().map(|loc| loc.line())
                }));
            }
            panic::catch_unwind(f).unwrap_err();
            panic::set_hook(hook);
            let x = loc.lock().unwrap().unwrap();
            x
        }

        fn main() {
            assert_eq!(panicked_at(|| block_on(foo())), 46);

            #[cfg(afn)]
            assert_eq!(panicked_at(|| block_on(foo_track_caller())), 61);
            #[cfg(any(cls, nofeat))]
            assert_eq!(panicked_at(|| block_on(foo_track_caller())), 57);

            #[cfg(afn)]
            assert_eq!(panicked_at(|| block_on(foo_assoc())), 76);
            #[cfg(any(cls, nofeat))]
            assert_eq!(panicked_at(|| block_on(foo_assoc())), 71);

            #[cfg(cls)]
            assert_eq!(panicked_at(|| block_on(foo_closure())), 84);

            #[cfg(cls)]
            assert_eq!(panicked_at(|| block_on(foo_block())), 96);
        }
    }

    // rust/tests/ui/async-await/deep-futures-are-freeze.rs

    fn _deep_futures_are_freeze() {
        // no-prefer-dynamic

        #![recursion_limit = "256"]

        fn main() {
            spawn(move || main0())
        }

        fn spawn<F>(future: impl FnOnce() -> F) {
            future();
        }

        async fn main0() {
            main1().await;
            main2().await;
        }
        async fn main1() {
            main2().await;
            main3().await;
        }
        async fn main2() {
            main3().await;
            main4().await;
        }
        async fn main3() {
            main4().await;
            main5().await;
        }
        async fn main4() {
            main5().await;
            main6().await;
        }
        async fn main5() {
            main6().await;
            main7().await;
        }
        async fn main6() {
            main7().await;
            main8().await;
        }
        async fn main7() {
            main8().await;
            main9().await;
        }
        async fn main8() {
            main9().await;
            main10().await;
        }
        async fn main9() {
            main10().await;
            main11().await;
        }
        async fn main10() {
            main11().await;
            main12().await;
        }
        async fn main11() {
            main12().await;
            main13().await;
        }
        async fn main12() {
            main13().await;
            main14().await;
        }
        async fn main13() {
            main14().await;
            main15().await;
        }
        async fn main14() {
            main15().await;
            main16().await;
        }
        async fn main15() {
            main16().await;
            main17().await;
        }
        async fn main16() {
            main17().await;
            main18().await;
        }
        async fn main17() {
            main18().await;
            main19().await;
        }
        async fn main18() {
            main19().await;
            main20().await;
        }
        async fn main19() {
            main20().await;
            main21().await;
        }
        async fn main20() {
            main21().await;
            main22().await;
        }
        async fn main21() {
            main22().await;
            main23().await;
        }
        async fn main22() {
            main23().await;
            main24().await;
        }
        async fn main23() {
            main24().await;
            main25().await;
        }
        async fn main24() {
            main25().await;
            main26().await;
        }
        async fn main25() {
            main26().await;
            main27().await;
        }
        async fn main26() {
            main27().await;
            main28().await;
        }
        async fn main27() {
            main28().await;
            main29().await;
        }
        async fn main28() {
            main29().await;
            main30().await;
        }
        async fn main29() {
            main30().await;
            main31().await;
        }
        async fn main30() {
            main31().await;
            main32().await;
        }
        async fn main31() {
            main32().await;
            main33().await;
        }
        async fn main32() {
            main33().await;
            main34().await;
        }
        async fn main33() {
            main34().await;
            main35().await;
        }
        async fn main34() {
            main35().await;
            main36().await;
        }
        async fn main35() {
            main36().await;
            main37().await;
        }
        async fn main36() {
            main37().await;
            main38().await;
        }
        async fn main37() {
            main38().await;
            main39().await;
        }
        async fn main38() {
            main39().await;
            main40().await;
        }
        async fn main39() {
            main40().await;
        }
        async fn main40() {
            boom(&mut ()).await;
        }

        async fn boom(f: &mut ()) {}
    }

    // rust/tests/ui/async-await/generics-and-bounds.rs

    fn _generics_and_bounds() {
        use std::future::Future;

        pub async fn simple_generic<T>() {}

        pub trait Foo {
            fn foo(&self) {}
        }

        struct FooType;
        impl Foo for FooType {}

        pub async fn call_generic_bound<F: Foo>(f: F) {
            f.foo()
        }

        pub async fn call_where_clause<F>(f: F)
        where
            F: Foo,
        {
            f.foo()
        }

        pub async fn call_impl_trait(f: impl Foo) {
            f.foo()
        }

        pub async fn call_with_ref(f: &impl Foo) {
            f.foo()
        }

        pub fn async_fn_with_same_generic_params_unifies() {
            let mut a = call_generic_bound(FooType);
            a = call_generic_bound(FooType);

            let mut b = call_where_clause(FooType);
            b = call_where_clause(FooType);

            let mut c = call_impl_trait(FooType);
            c = call_impl_trait(FooType);

            let f_one = FooType;
            let f_two = FooType;
            let mut d = call_with_ref(&f_one);
            d = call_with_ref(&f_two);
        }

        pub fn simple_generic_block<T>() -> impl Future<Output = ()> {
            async move {}
        }

        pub fn call_generic_bound_block<F: Foo>(f: F) -> impl Future<Output = ()> {
            async move { f.foo() }
        }

        pub fn call_where_clause_block<F>(f: F) -> impl Future<Output = ()>
        where
            F: Foo,
        {
            async move { f.foo() }
        }

        pub fn call_impl_trait_block(f: impl Foo) -> impl Future<Output = ()> {
            async move { f.foo() }
        }

        pub fn call_with_ref_block<'a>(f: &'a (impl Foo + 'a)) -> impl Future<Output = ()> + 'a {
            async move { f.foo() }
        }

        pub fn async_block_with_same_generic_params_unifies() {
            let mut a = call_generic_bound_block(FooType);
            a = call_generic_bound_block(FooType);

            let mut b = call_where_clause_block(FooType);
            b = call_where_clause_block(FooType);

            let mut c = call_impl_trait_block(FooType);
            c = call_impl_trait_block(FooType);

            let f_one = FooType;
            let f_two = FooType;
            let mut d = call_with_ref_block(&f_one);
            d = call_with_ref_block(&f_two);
        }
    }

    // rust/tests/ui/async-await/issue-105501.rs

    fn _issue_105501() {
        // This is a regression test for https://github.com/rust-lang/rust/issues/105501.
        // It was minified from the published `msf-ice:0.2.1` crate which failed in a crater run.
        // A faulty compiler was triggering a `higher-ranked lifetime error`:
        //
        // > could not prove `[async block@...]: Send`

        use mini_futures::Stream;

        fn is_send(_: impl Send) {}

        pub fn main() {
            let fut = async {
                let mut stream = mini_futures::iter([()])
                    .then(|_| async {})
                    .map(|_| async { None })
                    .buffered()
                    .filter_map(std::future::ready);

                stream.next().await
            };

            is_send(async move {
                let _: Option<()> = fut.await;
            });
        }

        // this is a simplified subset of `futures::StreamExt` and related types
        mod mini_futures {
            use std::future::Future;
            use std::pin::Pin;
            use std::task::{Context, Poll};

            pub fn iter<I>(_: I) -> Iter<I::IntoIter>
            where
                I: IntoIterator,
            {
                todo!()
            }

            pub trait Stream {
                type Item;

                fn then<Fut, F>(self, _: F) -> Then<Self, Fut, F>
                where
                    F: FnMut(Self::Item) -> Fut,
                    Fut: Future,
                    Self: Sized,
                {
                    todo!()
                }

                fn map<T, F>(self, _: F) -> Map<Self, F>
                where
                    F: FnMut(Self::Item) -> T,
                    Self: Sized,
                {
                    todo!()
                }

                fn buffered(self) -> Buffered<Self>
                where
                    Self::Item: Future,
                    Self: Sized,
                {
                    todo!()
                }

                fn filter_map<Fut, T, F>(self, _: F) -> FilterMap<Self, Fut, F>
                where
                    F: FnMut(Self::Item) -> Fut,
                    Fut: Future<Output = Option<T>>,
                    Self: Sized,
                {
                    todo!()
                }

                fn next(&mut self) -> Next<'_, Self> {
                    todo!()
                }
            }

            pub struct Iter<I> {
                __: I,
            }
            impl<I> Stream for Iter<I>
            where
                I: Iterator,
            {
                type Item = I::Item;
            }

            pub struct Then<St, Fut, F> {
                __: (St, Fut, F),
            }
            impl<St, Fut, F> Stream for Then<St, Fut, F>
            where
                St: Stream,
                F: FnMut(St::Item) -> Fut,
                Fut: Future,
            {
                type Item = Fut::Output;
            }

            pub struct Map<St, F> {
                __: (St, F),
            }
            impl<St, F> Stream for Map<St, F>
            where
                St: Stream,
                F: FnMut1<St::Item>,
            {
                type Item = F::Output;
            }

            pub trait FnMut1<A> {
                type Output;
            }
            impl<T, A, R> FnMut1<A> for T
            where
                T: FnMut(A) -> R,
            {
                type Output = R;
            }

            pub struct Buffered<St>
            where
                St: Stream,
                St::Item: Future,
            {
                __: (St, St::Item),
            }
            impl<St> Stream for Buffered<St>
            where
                St: Stream,
                St::Item: Future,
            {
                type Item = <St::Item as Future>::Output;
            }

            pub struct FilterMap<St, Fut, F> {
                __: (St, Fut, F),
            }
            impl<St, Fut, F, T> Stream for FilterMap<St, Fut, F>
            where
                St: Stream,
                F: FnMut1<St::Item, Output = Fut>,
                Fut: Future<Output = Option<T>>,
            {
                type Item = T;
            }

            pub struct Next<'a, St: ?Sized> {
                __: &'a mut St,
            }
            impl<St: ?Sized + Stream> Future for Next<'_, St> {
                type Output = Option<St::Item>;

                fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                    todo!()
                }
            }
        }
    }
}
