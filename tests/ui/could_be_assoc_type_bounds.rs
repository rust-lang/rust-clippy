//@aux-build:proc_macros.rs
#![allow(clippy::extra_unused_type_parameters)]

extern crate proc_macros;

fn projection_with_existing_assoc_bounds<T>()
where
    T: Iterator<Item: Clone>,
    <T as Iterator>::Item: Copy + Sized,
    //~^ could_be_assoc_type_bounds
{
}

fn projection_with_existing_bounds<T: Iterator<Item: Clone>>()
where
    <T as Iterator>::Item: Copy + Sized,
    //~^ could_be_assoc_type_bounds
{
}

fn no_fully_qualified_path<T: Iterator<Item: Clone>>()
where
    // False negative for now: `T::Item` has a `Res::Err` resolution
    T::Item: Copy + Sized,
{
}

fn ty_param<T: Iterator<Item = P>, P: Clone>() {}

fn multiple_projections<T>()
where
    T: Iterator,
    <T as Iterator>::Item: Sized,
    //~^ could_be_assoc_type_bounds
    <T as Iterator>::Item: Clone,
{
}

fn ty_param_used_in_body<T: Iterator<Item = P>, P: Clone + Default>() {
    P::default();
}

fn nested_impl_trait(_: impl Iterator<Item = impl Sized>) {}

fn impl_trait_generic<T: Copy>(_: impl Iterator<Item = T>) {}

fn single_impl_trait(_: impl Iterator<Item = ()>) {}

fn parenthesized<T: Iterator<Item = F>, F: Fn()>() {} //~ could_be_assoc_type_bounds

// Make sure implicit generic lifetime parameters for delim doesn't mess up spans
pub fn elided_lifetime<I, T>(iter: I, delim: &str)
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Display, //~ could_be_assoc_type_bounds
{
}

fn parenthesized2<F: Fn()>()
where
    F::Output: Copy,
{
}
fn many_ty_params<T, U: Copy, X>()
//~^ could_be_assoc_type_bounds
where
    T: Iterator<Item = U>,
{
}

#[clippy::msrv = "1.78.0"]
fn low_msrv<T: Iterator<Item = P>, P: Copy + Default>() {
    #[clippy::msrv = "1.79.0"]
    P::default();
}

// More involved test case with multiple associated types and generic parameters
trait Trait1<G1, G2>: Default {
    type A2;
    type A3;
    type A4;
}

fn complex<T, U, G1, G2>()
where
    (T, T): Trait1<G1, G2, A2 = u32, A3 = U>,
    <(T, T) as Trait1<G1, G2>>::A4: Clone,
    //~^ could_be_assoc_type_bounds
    U: Clone,
{
}

proc_macros::external! {
    fn external<T: Iterator<Item = I>, I: Copy>() {}
}
proc_macros::with_span! {
    span
    fn external2<T: Iterator<Item = I>, I: Copy>() {}
}

fn main() {}
