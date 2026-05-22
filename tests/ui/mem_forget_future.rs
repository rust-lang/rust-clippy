#![warn(clippy::mem_forget_future)]

use core::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

fn main() {
    let fut = foo();
    mem::forget(fut);
    //~^ mem_forget_future

    mem::forget(async {});
    //~^ mem_forget_future

    let fut = MyFuture;
    mem::forget(fut);
    //~^ mem_forget_future
}

async fn foo() {}

struct MyFuture;

impl Future for MyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

impl Drop for MyFuture {
    fn drop(&mut self) {}
}
