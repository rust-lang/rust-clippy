//@aux-build:proc_macros.rs
#![warn(clippy::unnecessary_box_pin)]

extern crate proc_macros;

use std::convert::identity;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;

async fn fut() {}

fn accept_unpin_fut(_: impl Future + Unpin) {}

fn assert_static(_: impl Send + Sync + 'static) {}

fn test(cx: &mut Context<'_>) {
    accept_unpin_fut(Box::pin(fut()).as_mut());
    //~^ unnecessary_box_pin

    let mut bx = Box::pin(fut());
    //~^ unnecessary_box_pin
    accept_unpin_fut(bx.as_mut());

    #[allow(clippy::let_underscore_future)]
    let _: Pin<&mut _> = Box::pin(fut()).as_mut();
    //~^ unnecessary_box_pin

    let bx = Box::pin(fut());
    assert_static(|| bx);
    assert_static(|| Box::pin(fut()));

    Box::pin(fut()).as_mut().poll(cx);
    //~^ unnecessary_box_pin

    assert_static(identity(Box::pin(async {})));

    let mut bx = Box::pin(fut());
    //~^ unnecessary_box_pin
    loop {
        bx.as_mut().poll(cx);
    }

    proc_macros::with_span! {
        span
        let mut bx = Box::pin(fut());
        accept_unpin_fut(bx.as_mut());
    }
    proc_macros::external! {
        let mut bx = Box::pin(fut());
        accept_unpin_fut(bx.as_mut());
    }
}

#[clippy::msrv = "1.67.0"]
fn too_low_msrv() {
    let mut bx = Box::pin(fut());
    accept_unpin_fut(bx.as_mut());
}

fn main() {}
