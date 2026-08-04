#![warn(clippy::match_like_matches_macro)]
#![allow(
    irrefutable_let_patterns,
    clippy::redundant_guards,
    clippy::unneeded_wildcard_pattern
)]
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

    // TODO: This can be transformed into `matches!(match_type, MatchType::A(_) | MatchType::C)`,
    // but is currently not linted because the middle arm returns a different boolean value.
    fn match_check_todo(match_type: MatchType) -> bool {
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

    fn tuple_binding_names(v: (Option<u8>, Option<u8>)) -> bool {
        match v {
            (Some(left), _) => true,
            (_, Some(right)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn ref_binding_names(v: (Option<u8>, Option<u8>)) -> bool {
        match v {
            (Some(ref x), Some(1)) => true,
            (Some(ref x), Some(2)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn ref_binding_names2(v1: Option<u8>, v2: Option<u8>) -> bool {
        match (v1, v2) {
            (Some(ref x), Some(1)) => true,
            (Some(ref x), Some(2)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn ref_binding_names_three_arms(v: (Option<u8>, u8)) -> bool {
        match v {
            (Some(ref x), 1) => true,
            (Some(ref x), 2) => true,
            (Some(ref x), 3) => true,
            _ => false,
        }
        //~^^^^^^ match_like_matches_macro
    }

    fn ref_binding_var_is_not_used(v: (Option<u8>, u8)) -> bool {
        match v {
            (Some(ref x), 1) => true,
            (Some(ref y), 2) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn ref_binding_multi_arm_var_is_not_used(v: (Option<u8>, u8)) -> bool {
        match v {
            (Some(ref x), 1) => true,
            (Some(ref x), 2) => true,
            (Some(ref z), 3) => true,
            _ => false,
        }
        //~^^^^^^ match_like_matches_macro
    }

    fn ref_mut_binding_names(v: (&mut Option<u8>, &mut Option<u8>)) -> bool {
        match v {
            (&mut Some(ref mut x), &mut Some(1)) => true,
            (&mut Some(ref mut x), &mut Some(2)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn ref_mut_binding_names2(v1: &mut Option<u8>, v2: &mut Option<u8>) -> bool {
        match (v1, v2) {
            (&mut Some(ref mut x), &mut Some(1)) => true,
            (&mut Some(ref mut x), &mut Some(2)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    fn mut_binding(v: (Option<u8>, Option<u8>)) -> bool {
        match v {
            (Some(mut x), Some(1)) => true,
            (Some(mut x), Some(2)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct StructRefBinding {
        x: Option<u8>,
        y: u8,
    }
    fn struct_ref_binding(v: StructRefBinding) -> bool {
        match v {
            StructRefBinding { ref x, y: 1 } => true,
            StructRefBinding { ref x, y: 2 } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }
    fn struct_ref_binding2(v: StructRefBinding) -> bool {
        match v {
            StructRefBinding { x: ref a, y: 1 } => true,
            StructRefBinding { x: ref b, y: 2 } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct TestBindingNames {
        a: u8,
        b: u8,
    }
    fn struct_binding_names(x: TestBindingNames) -> bool {
        match x {
            TestBindingNames { a, .. } => true,
            TestBindingNames { b, .. } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct TestBindingNames2 {
        a: u8,
        b: u8,
        c: u8,
    }
    fn struct_binding_names2(x: TestBindingNames2) -> bool {
        match x {
            TestBindingNames2 { a, b: 1, .. } => true,
            TestBindingNames2 { a, b: 2, .. } => true,
            TestBindingNames2 { a: 3, b: 4, .. } => true,
            _ => false,
        }
        //~^^^^^^ match_like_matches_macro
    }

    struct MixedBindingNames {
        a: u8,
        b: u8,
    }
    fn mixed_binding_names(x: MixedBindingNames, y: Option<u8>) -> bool {
        match (x, y) {
            (MixedBindingNames { a, .. }, Some(x)) => true,
            (MixedBindingNames { b, .. }, Some(y)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct MixedNested {
        a: Option<u8>,
        b: u8,
    }
    fn mixed_nested(x: MixedNested) -> bool {
        match x {
            MixedNested { a: Some(x), b } => true,
            MixedNested { a: Some(y), b: c } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct MixedFields {
        a: u8,
        b: u8,
    }
    fn mixed_fields(x: MixedFields) -> bool {
        match x {
            MixedFields { a, b: 1 } => true,
            MixedFields { a: 2, b } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    struct InnerStruct {
        x: u8,
    }
    struct OuterStruct {
        inner: InnerStruct,
        y: u8,
    }
    fn nested_struct(x: OuterStruct) -> bool {
        match x {
            OuterStruct {
                inner: InnerStruct { x, .. },
                y: w,
            } => true,
            OuterStruct {
                inner: InnerStruct { x: z, .. },
                y: w,
            } => true,
            _ => false,
        }
        //~^^^^^^^^^^^ match_like_matches_macro
    }
    fn nested_struct_shorthand(x: OuterStruct) -> bool {
        match x {
            OuterStruct {
                inner: InnerStruct { x, .. },
                y,
            } => true,
            OuterStruct {
                inner: InnerStruct { x, .. },
                y,
            } => true,
            _ => false,
        }
        //~^^^^^^^^^^^ match_like_matches_macro
    }
    fn nested_struct_ref_shorthand(v: OuterStruct) -> bool {
        match v {
            OuterStruct {
                inner: InnerStruct { ref x },
                y: 1,
            } => true,
            OuterStruct {
                inner: InnerStruct { ref x },
                y: 2,
            } => true,
            _ => false,
        }
        //~^^^^^^^^^^^ match_like_matches_macro
    }

    struct AdjacentShorthand {
        a: u8,
        b: u8,
    }
    fn adjacent_shorthand_fields(x: AdjacentShorthand) -> bool {
        match x {
            AdjacentShorthand { a, b } => true,
            AdjacentShorthand { a: 1, b: 2 } => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

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

    // Bail on at bindings, so this one won't be linted.
    fn at_binding(v: Option<u8>) -> bool {
        match v {
            Some(x @ 0) => true,
            Some(y @ 1) => true,
            _ => false,
        }
    }

    fn slice_binding(v: &[u8]) -> bool {
        match v {
            [first, ..] => true,
            [_, last] => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }

    macro_rules! mk_pat {
        ($name:ident) => {
            Some($name)
        };
    }
    enum FromExpansion {
        A(Option<u8>),
        B(Option<u8>),
    }

    // ok
    fn from_expansion(v: Option<u8>) -> bool {
        match v {
            mk_pat!(a) => true,
            mk_pat!(b) => true,
            _ => false,
        }
    }

    // ok
    fn from_expansion2(v: FromExpansion) -> bool {
        match v {
            FromExpansion::A(mk_pat!(a)) => true,
            FromExpansion::B(mk_pat!(b)) => true,
            _ => false,
        }
    }

    struct Data {
        x: u8,
    }
    enum Kind {
        A(Data),
        B(Data),
    }
    // Test all of above cases in one function.
    fn everything(v: (Kind, Option<u8>)) -> bool {
        match v {
            (Kind::A(Data { x, .. }), Some(y)) => true,
            (Kind::B(Data { x: z, .. }), Some(w)) => true,
            _ => false,
        }
        //~^^^^^ match_like_matches_macro
    }
}
