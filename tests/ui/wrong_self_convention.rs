#![warn(clippy::wrong_self_convention)]

fn main() {}

#[derive(Clone, Copy)]
struct Foo;

impl Foo {
    fn as_i32(self) {}
    fn as_u32(&self) {}
    fn into_i32(self) {}
    fn is_i32(self) {}
    fn is_u32(&self) {}
    fn to_i32(self) {}
    fn from_i32(self) {}
    //~^ wrong_self_convention

    pub fn as_i64(self) {}
    pub fn into_i64(self) {}
    pub fn is_i64(self) {}
    pub fn to_i64(self) {}
    pub fn from_i64(self) {}
    //~^ wrong_self_convention

    // check whether the lint can be allowed at the function level
    #[allow(clippy::wrong_self_convention)]
    pub fn from_cake(self) {}

    fn as_x<F: AsRef<Self>>(_: F) {}
    fn as_y<F: AsRef<Foo>>(_: F) {}
}

struct Bar;

impl Bar {
    fn as_i32(self) {}
    //~^ wrong_self_convention

    fn as_u32(&self) {}
    fn into_i32(&self) {}
    //~^ wrong_self_convention

    fn into_u32(self) {}
    fn is_i32(self) {}
    //~^ wrong_self_convention

    fn is_u32(&self) {}
    fn to_i32(self) {}
    //~^ wrong_self_convention

    fn to_u32(&self) {}
    fn from_i32(self) {}
    //~^ wrong_self_convention

    pub fn as_i64(self) {}
    //~^ wrong_self_convention

    pub fn into_i64(&self) {}
    //~^ wrong_self_convention

    pub fn is_i64(self) {}
    //~^ wrong_self_convention

    pub fn to_i64(self) {}
    //~^ wrong_self_convention

    pub fn from_i64(self) {}
    //~^ wrong_self_convention

    // test for false positives
    fn as_(self) {}
    fn into_(&self) {}
    fn is_(self) {}
    fn to_(self) {}
    fn from_(self) {}
    fn to_mut(&mut self) {}
}

// `as_` conversions must return a borrowed view (borrowed -> borrowed, C-CONV),
// so returning an owned type is also a violation (#7676).
mod as_return_type {
    pub struct Target;

    impl Target {
        // borrowed return: correct, no lint
        pub fn as_bytes(&self) -> &'static [u8] {
            &[]
        }
        pub fn as_mut_bytes(&mut self) -> &'static mut [u8] {
            &mut []
        }
        // owned return: violates the convention
        pub fn as_owned(&self) -> Vec<u8> {
            //~^ wrong_self_convention
            Vec::new()
        }
        // a borrow inside another type is still borrowed, so it is not flagged
        pub fn as_result(&self) -> Result<&'static u8, &'static u8> {
            Ok(&0)
        }
        // unit return is not a conversion, so it is not flagged
        pub fn as_unit(&self) {}
    }
}

// Allow Box<Self>, Rc<Self>, Arc<Self> for methods that take conventionally take Self by value
#[allow(clippy::boxed_local)]
mod issue4293 {
    use std::rc::Rc;
    use std::sync::Arc;

    struct T;

    impl T {
        fn into_s1(self: Box<Self>) {}
        fn into_s2(self: Rc<Self>) {}
        fn into_s3(self: Arc<Self>) {}

        fn into_t1(self: Box<T>) {}
        fn into_t2(self: Rc<T>) {}
        fn into_t3(self: Arc<T>) {}
    }
}

// False positive for async (see #4037)
mod issue4037 {
    pub struct Foo;
    pub struct Bar;

    impl Foo {
        pub async fn into_bar(self) -> Bar {
            Bar
        }
    }
}

// Lint also in trait definition (see #6307)
mod issue6307 {
    trait T: Sized {
        fn as_i32(self) {}
        //~^ wrong_self_convention

        fn as_u32(&self) {}
        fn into_i32(self) {}
        fn into_i32_ref(&self) {}
        //~^ wrong_self_convention

        fn into_u32(self) {}
        fn is_i32(self) {}
        //~^ wrong_self_convention

        fn is_u32(&self) {}
        fn to_i32(self) {}
        fn to_u32(&self) {}
        fn from_i32(self) {}
        //~^ wrong_self_convention

        // check whether the lint can be allowed at the function level
        #[allow(clippy::wrong_self_convention)]
        fn from_cake(self) {}

        // test for false positives
        fn as_(self) {}
        fn into_(&self) {}
        fn is_(self) {}
        fn to_(self) {}
        fn from_(self) {}
        fn to_mut(&mut self) {}
    }

    trait U {
        fn as_i32(self);
        //~^ wrong_self_convention

        fn as_u32(&self);
        fn into_i32(self);
        fn into_i32_ref(&self);
        //~^ wrong_self_convention

        fn into_u32(self);
        fn is_i32(self);
        //~^ wrong_self_convention

        fn is_u32(&self);
        fn to_i32(self);
        fn to_u32(&self);
        fn from_i32(self);
        //~^ wrong_self_convention

        // check whether the lint can be allowed at the function level
        #[allow(clippy::wrong_self_convention)]
        fn from_cake(self);

        // test for false positives
        fn as_(self);
        fn into_(&self);
        fn is_(self);
        fn to_(self);
        fn from_(self);
        fn to_mut(&mut self);
    }

    trait C: Copy {
        fn as_i32(self);
        fn as_u32(&self);
        fn into_i32(self);
        fn into_i32_ref(&self);
        //~^ wrong_self_convention

        fn into_u32(self);
        fn is_i32(self);
        fn is_u32(&self);
        fn to_i32(self);
        fn to_u32(&self);
        fn from_i32(self);
        //~^ wrong_self_convention

        // check whether the lint can be allowed at the function level
        #[allow(clippy::wrong_self_convention)]
        fn from_cake(self);

        // test for false positives
        fn as_(self);
        fn into_(&self);
        fn is_(self);
        fn to_(self);
        fn from_(self);
        fn to_mut(&mut self);
    }
}

mod issue6727 {
    #[derive(Clone, Copy)]
    struct FooCopy;

    impl FooCopy {
        fn to_u64(self) -> u64 {
            1
        }
        // trigger lint
        fn to_u64_v2(&self) -> u64 {
            //~^ wrong_self_convention

            1
        }
    }

    struct FooNoCopy;

    impl FooNoCopy {
        // trigger lint
        fn to_u64(self) -> u64 {
            //~^ wrong_self_convention

            2
        }
        fn to_u64_v2(&self) -> u64 {
            2
        }
    }
}

pub mod issue8142 {
    struct S;

    impl S {
        // Should not lint: "no self at all" is allowed.
        fn is_forty_two(x: u32) -> bool {
            x == 42
        }

        // Should not lint: &self is allowed.
        fn is_test_code(&self) -> bool {
            true
        }
    }
}
