#![warn(clippy::missing_must_use_on_future_types)]
#![allow(path_statements, unused, clippy::no_effect)]

use std::fmt::Display;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

// basic struct case
struct BasicStruct;

impl Future for BasicStruct {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

// basic enum case
enum BasicEnum {
    Var1,
    Var2,
}

impl Future for BasicEnum {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

// should be ignored if type doesn't implement `Future`
struct NonFuture;

// should still trigger if a type only sometimes implements `Future`
struct SometimesFuture<T>(PhantomData<T>);

impl<T: Copy> Future for SometimesFuture<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

// should be ignored on trait objects
trait Trait {}

impl Future for dyn Trait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

struct TraitImpl {}
impl Trait for TraitImpl {}

fn trait_obj() -> Box<dyn Trait> {
    Box::new(TraitImpl {})
}

// struct with multiple fields and impls
#[derive(Debug)]
pub struct ComplexStruct {
    x: usize,
    y: &'static str,
}

impl ComplexStruct {
    fn sum(&self) -> usize {
        self.x + self.y.len()
    }
}

impl Display for ComplexStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "complex")
    }
}

impl Future for ComplexStruct {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

// should be ignored on already #[must_use] struct
#[must_use]
struct AlreadyMustUse;

impl Future for AlreadyMustUse {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

fn main() {
    BasicStruct;
    BasicEnum::Var2;
    NonFuture;
    SometimesFuture::<String>(PhantomData);
    trait_obj();
    ComplexStruct { x: 42, y: "complex" };
    AlreadyMustUse;
}
