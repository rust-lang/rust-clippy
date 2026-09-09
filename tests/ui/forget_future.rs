#![warn(clippy::forget_future)]

use core::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

fn main() {
    let fut = foo();
    mem::forget(fut);
    //~^ forget_future

    mem::forget(async {});
    //~^ forget_future

    let fut = MyFuture;
    mem::forget(fut);
    //~^ forget_future

    #[expect(clippy::forget_non_drop)]
    mem::forget(NoDropFuture);
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

struct NoDropFuture;

impl Future for NoDropFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}
