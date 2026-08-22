#![warn(clippy::match_like_matches_macro)]
#![allow(irrefutable_let_patterns, clippy::redundant_guards)]
#![expect(clippy::needless_borrowed_reference)]

fn main() {
    let x = Some(5);

    // Lint
    let _y = match x {
        Some(0) => true,
        _ => false,
    };
    //~^^^^ match_like_matches_macro

    // No lint: covered by `redundant_pattern_matching`
    let _w = match x {
        Some(_) => true,
        _ => false,
    };
    //~^^^^ redundant_pattern_matching

    // No lint: covered by `redundant_pattern_matching`
    let _z = match x {
        Some(_) => false,
        None => true,
    };
    //~^^^^ redundant_pattern_matching

    // Lint
    let _zz = match x {
        Some(r) if r == 0 => false,
        _ => true,
    };
    //~^^^^ match_like_matches_macro

    // Lint
    let _zzz = if let Some(5) = x { true } else { false };
    //~^ match_like_matches_macro

    // No lint
    let _a = match x {
        Some(_) => false,
        _ => false,
    };

    // No lint
    let _ab = match x {
        Some(0) => false,
        _ => true,
        None => false,
    };

    enum E {
        A(u32),
        B(i32),
        C,
        D,
    }
    let x = E::A(2);
    {
        // lint
        let _ans = match x {
            E::A(_) => true,
            E::B(_) => true,
            _ => false,
        };
        //~^^^^^ match_like_matches_macro
    }
    {
        // lint
        // skip rustfmt to prevent removing block for first pattern
        #[rustfmt::skip]
        let _ans = match x {
            E::A(_) => {
                true
            }
            E::B(_) => true,
            _ => false,
        };
        //~^^^^^^^ match_like_matches_macro
    }
    {
        // lint
        let _ans = match x {
            E::B(_) => false,
            E::C => false,
            _ => true,
        };
        //~^^^^^ match_like_matches_macro
    }
    {
        // no lint
        let _ans = match x {
            E::A(_) => false,
            E::B(_) => false,
            E::C => true,
            _ => true,
        };
    }
    {
        // no lint
        let _ans = match x {
            E::A(_) => true,
            E::B(_) => false,
            E::C => false,
            _ => true,
        };
    }
    {
        // no lint
        let _ans = match x {
            E::A(a) if a < 10 => false,
            E::B(a) if a < 10 => false,
            _ => true,
        };
    }
    {
        // no lint
        let _ans = match x {
            E::A(_) => false,
            E::B(a) if a < 10 => false,
            _ => true,
        };
    }
    {
        // no lint
        let _ans = match x {
            E::A(a) => a == 10,
            E::B(_) => false,
            _ => true,
        };
    }
    {
        // no lint
        let _ans = match x {
            E::A(_) => false,
            E::B(_) => true,
            _ => false,
        };
    }

    {
        // should print "z" in suggestion (#6503)
        let z = &Some(3);
        let _z = match &z {
            Some(3) => true,
            _ => false,
        };
        //~^^^^ match_like_matches_macro
    }

    {
        // this could also print "z" in suggestion..?
        let z = Some(3);
        let _z = match &z {
            Some(3) => true,
            _ => false,
        };
        //~^^^^ match_like_matches_macro
    }

    {
        enum AnEnum {
            X,
            Y,
        }

        fn foo(_x: AnEnum) {}

        fn main() {
            let z = AnEnum::X;
            // we can't remove the reference here!
            let _ = match &z {
                AnEnum::X => true,
                _ => false,
            };
            //~^^^^ match_like_matches_macro
            foo(z);
        }
    }

    {
        struct S(i32);

        fn fun(_val: Option<S>) {}
        let val = Some(S(42));
        // we need the reference here because later val is consumed by fun()
        let _res = match &val {
            &Some(ref _a) => true,
            _ => false,
        };
        //~^^^^ match_like_matches_macro
        fun(val);
    }

    {
        struct S(i32);

        fn fun(_val: Option<S>) {}
        let val = Some(S(42));
        let _res = match &val {
            &Some(ref _a) => true,
            _ => false,
        };
        //~^^^^ match_like_matches_macro
        fun(val);
    }

    {
        enum E {
            A,
            B,
            C,
        }

        let _ = match E::A {
            E::B => true,
            #[cfg(feature = "foo")]
            E::A => true,
            _ => false,
        };
    }

    let x = ' ';
    // ignore if match block contains comment
    let _line_comments = match x {
        // numbers are bad!
        '1' | '2' | '3' => true,
        // spaces are very important to be true.
        ' ' => true,
        // as are dots
        '.' => true,
        _ => false,
    };

    let _block_comments = match x {
        /* numbers are bad!
         */
        '1' | '2' | '3' => true,
        /* spaces are very important to be true.
         */
        ' ' => true,
        /* as are dots
         */
        '.' => true,
        _ => false,
    };
}

#[clippy::msrv = "1.41"]
fn msrv_1_41() {
    let _y = match Some(5) {
        Some(0) => true,
        _ => false,
    };
}

#[clippy::msrv = "1.42"]
fn msrv_1_42() {
    let _y = match Some(5) {
        Some(0) => true,
        _ => false,
    };
    //~^^^^ match_like_matches_macro
}

#[expect(clippy::option_option)]
fn issue15841(opt: Option<Option<Option<i32>>>, value: i32) {
    // Lint: no if-let _in the guard_
    let _ = match opt {
        Some(first) if (if let Some(second) = first { true } else { todo!() }) => true,
        _ => false,
    };
    //~^^^^ match_like_matches_macro
}

fn issue16015<T: 'static, U: 'static>() -> bool {
    use std::any::{TypeId, type_name};
    pub struct GetTypeId<T>(T);

    impl<T: 'static> GetTypeId<T> {
        pub const VALUE: TypeId = TypeId::of::<T>();
    }

    macro_rules! typeid {
        ($t:ty) => {
            GetTypeId::<$t>::VALUE
        };
    }

    match typeid!(T) {
        _ => true,
        _ => false,
    };
    //~^^^^ match_like_matches_macro

    if let _ = typeid!(U) { true } else { false }
    //~^ match_like_matches_macro
}

mod issue17503 {
    enum MatchType {
        A(String),
        B(String, String),
        C,
    }

    fn matches(match_type: MatchType) -> bool {
        match match_type {
            MatchType::A(_str1) => true,
            MatchType::B(_str1, _str2) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn match_check_pass(match_type: MatchType) -> bool {
        match match_type {
            MatchType::A(_str1) => true,
            MatchType::B(_str1, _str2) => false,
            MatchType::C => true,
        }
    }

    fn different_binding_names(match_type: MatchType) -> bool {
        match match_type {
            MatchType::A(a) => true,
            MatchType::B(b, c) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn different_binding_names_2(match_type: MatchType) -> bool {
        match match_type {
            MatchType::A(r#match) => true,
            MatchType::B(very_long_name_in_snake_case, __very_strange_name__) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    enum MatchType2 {
        A,
        B,
        C,
    }

    fn matches2(match_type2: MatchType2) -> bool {
        match match_type2 {
            MatchType2::A => true,
            MatchType2::B => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn tuple_binding_names(v: (Option<u8>, Option<u8>)) -> bool {
        match v {
            (Some(left), _) => true,
            (_, Some(right)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    // FIXME: Erasing bindings in struct shorthand patterns (`Foo { a, .. }`)
    // requires rewriting them as `Foo { a: _, .. }`. Replacing only the binding
    // span currently produces invalid syntax (`Foo { _, .. }`).
    // ```rs
    // struct TestBindingNames {
    //     a: u8,
    //     b: u8,
    // }
    // fn struct_binding_names(x: TestBindingNames) -> bool {
    //     match x {
    //         TestBindingNames { a, .. } => true,
    //         TestBindingNames { b, .. } => true,
    //         _ => false,
    //     }
    // }
    // ```

    enum NestedTupleInEnum {
        A((u8, u8)),
        B((u8, u8)),
    }

    fn nested_tuple(e: NestedTupleInEnum) -> bool {
        match e {
            NestedTupleInEnum::A((x, y)) => true,
            NestedTupleInEnum::B((a, b)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    enum Inner {
        X(u8),
    }

    enum Outer {
        A(Inner),
        B(Inner),
    }

    fn nested_enum(o: Outer) -> bool {
        match o {
            Outer::A(Inner::X(x)) => true,
            Outer::B(Inner::X(y)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    // FIXME: Avoid invalid suggestions for `@` bindings.
    // ```rs
    // fn at_binding(v: Option<u8>) -> bool {
    //     match v {
    //         Some(x @ 0) => true,
    //         Some(y @ 1) => true,
    //         _ => false,
    //     }
    // }
    // ```

    fn slice_binding(v: &[u8]) -> bool {
        match v {
            [first, ..] => true,
            [_, last] => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }
}
