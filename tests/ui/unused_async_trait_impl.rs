#![warn(clippy::unused_async_trait_impl)]

trait HasAsyncMethod {
    async fn do_something() -> u32;
}

struct Inefficient;
struct Efficient;
struct Stub;

impl HasAsyncMethod for Inefficient {
    async fn do_something() -> u32 {
        //~^ unused_async_trait_impl
        1
    }
}

impl HasAsyncMethod for Efficient {
    fn do_something() -> impl Future<Output = u32> {
        core::future::ready(1)
    }
}

impl HasAsyncMethod for Stub {
    async fn do_something() -> u32 {
        todo!() // Do not emit the lint in this case.
    }
}
