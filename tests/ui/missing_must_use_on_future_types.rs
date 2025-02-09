//@aux-build:proc_macros.rs

#![warn(clippy::missing_must_use_on_future_types)]
#![allow(path_statements, unused, clippy::no_effect)]

extern crate proc_macros;
use proc_macros::{external, with_span};

use std::fmt::Display;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

struct BasicStruct; //~ ERROR: missing a `#[must_use]` attribute

impl Future for BasicStruct {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

enum BasicEnum {
    //~^ ERROR: missing a `#[must_use]` attribute
    Var1,
    Var2,
}

impl Future for BasicEnum {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

struct NonFuture;

struct SometimesFuture<T>(PhantomData<T>); //~ ERROR: missing a `#[must_use]` attribute

impl<T: Copy> Future for SometimesFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

#[derive(Debug)]
pub struct DeriveStruct; //~ ERROR: missing a `#[must_use]` attribute

impl Future for DeriveStruct {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

#[must_use]
struct AlreadyMustUse;

impl Future for AlreadyMustUse {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

external! {
    struct IgnoreExternal;

    impl Future for IgnoreExternal {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(())
        }
    }
}

with_span! { span
    struct LocalMacro;

    impl Future for LocalMacro {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(())
        }
    }
}
