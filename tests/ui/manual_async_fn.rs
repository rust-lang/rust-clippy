#![warn(clippy::manual_async_fn)]
#![allow(clippy::needless_pub_self, unused)]

use std::future::Future;

//~v manual_async_fn
fn fut() -> impl Future<Output = i32> {
    async { 42 }
}

#[rustfmt::skip]
//~v manual_async_fn
fn fut2() ->impl Future<Output = i32> {
    async { 42 }
}

#[rustfmt::skip]
//~v manual_async_fn
fn fut3()-> impl Future<Output = i32> {
    async { 42 }
}

//~v manual_async_fn
fn empty_fut() -> impl Future<Output = ()> {
    async {}
}

#[rustfmt::skip]
//~v manual_async_fn
fn empty_fut2() ->impl Future<Output = ()> {
    async {}
}

#[rustfmt::skip]
//~v manual_async_fn
fn empty_fut3()-> impl Future<Output = ()> {
    async {}
}

//~v manual_async_fn
fn core_fut() -> impl core::future::Future<Output = i32> {
    async move { 42 }
}

// should be ignored
fn has_other_stmts() -> impl core::future::Future<Output = i32> {
    let _ = 42;
    async move { 42 }
}

// should be ignored
fn not_fut() -> i32 {
    42
}

// should be ignored
async fn already_async() -> impl Future<Output = i32> {
    async { 42 }
}

struct S;
impl S {
    //~v manual_async_fn
    fn inh_fut() -> impl Future<Output = i32> {
        async {
            // NOTE: this code is here just to check that the indentation is correct in the suggested fix
            let a = 42;
            let b = 21;
            if a < b {
                let c = 21;
                let d = 42;
                if c < d {
                    let _ = 42;
                }
            }
            42
        }
    }

    // should be ignored
    fn not_fut(&self) -> i32 {
        42
    }

    // should be ignored
    fn has_other_stmts() -> impl core::future::Future<Output = i32> {
        let _ = 42;
        async move { 42 }
    }

    // should be ignored
    async fn already_async(&self) -> impl Future<Output = i32> {
        async { 42 }
    }
}

// Tests related to lifetime capture

//~v manual_async_fn
fn elided(_: &i32) -> impl Future<Output = i32> + '_ {
    async { 42 }
}

// should be ignored
fn elided_not_bound(_: &i32) -> impl Future<Output = i32> {
    async { 42 }
}

//~v manual_async_fn
fn explicit<'a, 'b>(_: &'a i32, _: &'b i32) -> impl Future<Output = i32> + 'a + 'b {
    async { 42 }
}

// should be ignored
#[allow(clippy::needless_lifetimes)]
fn explicit_not_bound<'a, 'b>(_: &'a i32, _: &'b i32) -> impl Future<Output = i32> {
    async { 42 }
}

// should be ignored
mod issue_5765 {
    use std::future::Future;

    struct A;
    impl A {
        fn f(&self) -> impl Future<Output = ()> {
            async {}
        }
    }

    fn test() {
        let _future = {
            let a = A;
            a.f()
        };
    }
}

//~v manual_async_fn
pub fn issue_10450() -> impl Future<Output = i32> {
    async { 42 }
}

//~v manual_async_fn
pub(crate) fn issue_10450_2() -> impl Future<Output = i32> {
    async { 42 }
}

//~v manual_async_fn
pub(self) fn issue_10450_3() -> impl Future<Output = i32> {
    async { 42 }
}

fn main() {}
