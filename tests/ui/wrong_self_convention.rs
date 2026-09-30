#![warn(clippy::wrong_self_convention)]

fn main() {}

#[derive(Clone, Copy)]
struct Foo;

impl Foo {
    fn as_i32(self) {}
    //~^ wrong_self_convention
    fn as_u32(&self) {}
    //~^ wrong_self_convention
    fn into_i32(self) {}
    //~^ wrong_self_convention
    fn is_i32(self) {}
    //~^ wrong_self_convention
    fn is_u32(&self) {}
    //~^ wrong_self_convention
    fn to_i32(self) {}
    fn from_i32(self) {}
    //~^ wrong_self_convention

    pub fn as_i64(self) {}
    //~^ wrong_self_convention
    pub fn into_i64(self) {}
    //~^ wrong_self_convention
    pub fn is_i64(self) {}
    //~^ wrong_self_convention
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
    //~^ wrong_self_convention
    fn into_i32(&self) {}
    //~^ wrong_self_convention

    fn into_u32(self) {}
    //~^ wrong_self_convention
    fn is_i32(self) {}
    //~^ wrong_self_convention

    fn is_u32(&self) {}
    //~^ wrong_self_convention
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

// Allow Box<Self>, Rc<Self>, Arc<Self> for methods that take conventionally take Self by value
#[allow(clippy::boxed_local)]
mod issue4293 {
    use std::rc::Rc;
    use std::sync::Arc;

    struct T;

    impl T {
        fn into_s1(self: Box<Self>) -> Box<Self> {
            self
        }
        fn into_s2(self: Rc<Self>) -> Rc<Self> {
            self
        }
        fn into_s3(self: Arc<Self>) -> Arc<Self> {
            self
        }

        fn into_t1(self: Box<T>) -> Box<T> {
            self
        }
        fn into_t2(self: Rc<T>) -> Rc<T> {
            self
        }
        fn into_t3(self: Arc<T>) -> Arc<T> {
            self
        }
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
        //~^ wrong_self_convention
        fn into_i32(self) {}
        //~^ wrong_self_convention
        fn into_i32_ref(&self) {}
        //~^ wrong_self_convention

        fn into_u32(self) {}
        //~^ wrong_self_convention
        fn is_i32(self) {}
        //~^ wrong_self_convention

        fn is_u32(&self) {}
        //~^ wrong_self_convention
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
        //~^ wrong_self_convention
        fn into_i32(self);
        //~^ wrong_self_convention
        fn into_i32_ref(&self);
        //~^ wrong_self_convention

        fn into_u32(self);
        //~^ wrong_self_convention
        fn is_i32(self);
        //~^ wrong_self_convention

        fn is_u32(&self);
        //~^ wrong_self_convention
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
        //~^ wrong_self_convention
        fn as_u32(&self);
        //~^ wrong_self_convention
        fn into_i32(self);
        //~^ wrong_self_convention
        fn into_i32_ref(&self);
        //~^ wrong_self_convention

        fn into_u32(self);
        //~^ wrong_self_convention
        fn is_i32(self);
        //~^ wrong_self_convention
        fn is_u32(&self);
        //~^ wrong_self_convention
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

// Return types of `as_` / `into_` / `is_` follow the naming convention (see #7676).
mod issue7676 {
    use std::pin::Pin;

    pub struct Target;
    pub struct Error;

    impl Target {
        fn as_bytes(&self) {}
        //~^ wrong_self_convention

        fn as_vec(&self) -> Vec<u8> {
            //~^ wrong_self_convention
            Vec::new()
        }

        fn as_u32(&self) -> u32 {
            //~^ wrong_self_convention
            0
        }

        fn as_ascii(&self) -> Option<u8> {
            //~^ wrong_self_convention
            None
        }

        fn into_vec(self) {}
        //~^ wrong_self_convention

        fn into_ref(self) -> &'static str {
            //~^ wrong_self_convention
            ""
        }

        fn into_borrowed(self) -> Result<&'static str, Error> {
            //~^ wrong_self_convention
            Ok("")
        }

        fn is_empty(&self) {}
        //~^ wrong_self_convention

        fn is_status(&self) -> u8 {
            //~^ wrong_self_convention
            0
        }

        fn as_slice(&self) -> &[u8] {
            &[]
        }
        fn as_mut_slice(&mut self) -> &mut [u8] {
            &mut []
        }
        fn as_ptr(&self) -> *const u8 {
            std::ptr::null()
        }
        fn as_mut_ptr(&mut self) -> *mut u8 {
            std::ptr::null_mut()
        }
        fn as_ref_opt(&self) -> Option<&Target> {
            Some(self)
        }
        fn as_result(&self) -> Result<&Target, &Error> {
            Ok(self)
        }
        fn as_pin(self: Pin<&Self>) -> Pin<&Self> {
            self
        }
        fn as_chunks(&self) -> (&[u8], &[u8]) {
            (&[], &[])
        }
        async fn as_bytes_async(&self) -> &[u8] {
            &[]
        }
        async fn as_owned(&self) -> Vec<u8> {
            //~^ wrong_self_convention
            Vec::new()
        }
        async fn as_nothing(&self) {}
        //~^ wrong_self_convention

        fn into_owned(self) -> Vec<u8> {
            Vec::new()
        }
        fn into_raw(self) -> *mut u8 {
            std::ptr::null_mut()
        }
        fn into_option(self) -> Option<Vec<u8>> {
            None
        }
        async fn into_vec_async(self) -> Vec<u8> {
            Vec::new()
        }

        fn is_empty_bool(&self) -> bool {
            true
        }

        fn as_generic<T: Default>(&self) -> T {
            T::default()
        }
        fn into_generic<T: Default>(self) -> T {
            T::default()
        }
    }
}
