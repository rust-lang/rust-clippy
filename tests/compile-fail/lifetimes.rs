#![feature(plugin)]
#![plugin(clippy)]

#![deny(needless_lifetimes)]
#![allow(dead_code)]
#![allow(unused)]
fn distinct_lifetimes<'a, 'b>(x: &'a u8, y: &'b u8, z: u8) { }
//~^ERROR explicit lifetimes given

fn distinct_and_static<'a, 'b>(x: &'a u8, y: &'b u8, z: &'static u8) { }
//~^ERROR explicit lifetimes given

fn same_lifetime_on_input<'a>(x: &'a u8, y: &'a u8) { } // no error, same lifetime on two params

fn only_static_on_input(x: &u8, y: &u8, z: &'static u8) { } // no error, static involved

fn mut_and_static_input(x: &mut u8, y: &'static str) { }

fn in_and_out<'a>(x: &'a u8, y: u8) -> &'a u8 { x }
//~^ERROR explicit lifetimes given

fn multiple_in_and_out_1<'a>(x: &'a u8, y: &'a u8) -> &'a u8 { x } // no error, multiple input refs

fn multiple_in_and_out_2<'a, 'b>(x: &'a u8, y: &'b u8) -> &'a u8 { x } // no error, multiple input refs

fn in_static_and_out<'a>(x: &'a u8, y: &'static u8) -> &'a u8 { x } // no error, static involved

fn deep_reference_1<'a, 'b>(x: &'a u8, y: &'b u8) -> Result<&'a u8, ()> { Ok(x) } // no error

fn deep_reference_2<'a>(x: Result<&'a u8, &'a u8>) -> &'a u8 { x.unwrap() } // no error, two input refs

fn deep_reference_3<'a>(x: &'a u8, y: u8) -> Result<&'a u8, ()> { Ok(x) }
//~^ERROR explicit lifetimes given

// where clause, but without lifetimes
fn where_clause_without_lt<'a, T>(x: &'a u8, y: u8) -> Result<&'a u8, ()> where T: Copy { Ok(x) }
//~^ERROR explicit lifetimes given

type Ref<'r> = &'r u8;

fn lifetime_param_1<'a>(x: Ref<'a>, y: &'a u8) { } // no error, same lifetime on two params

fn lifetime_param_2<'a, 'b>(x: Ref<'a>, y: &'b u8) { }
//~^ERROR explicit lifetimes given

fn lifetime_param_3<'a, 'b: 'a>(x: Ref<'a>, y: &'b u8) { } // no error, bounded lifetime

fn lifetime_param_4<'a, 'b>(x: Ref<'a>, y: &'b u8) where 'b: 'a { } // no error, bounded lifetime

struct Lt<'a, I: 'static> {
    x: &'a I
}

fn fn_bound<'a, F, I>(_m: Lt<'a, I>, _f: F) -> Lt<'a, I>
    where F: Fn(Lt<'a, I>) -> Lt<'a, I>  // no error, fn bound references 'a
{ unreachable!() }

fn fn_bound_2<'a, F, I>(_m: Lt<'a, I>, _f: F) -> Lt<'a, I>  //~ERROR explicit lifetimes given
    where for<'x> F: Fn(Lt<'x, I>) -> Lt<'x, I>
{ unreachable!() }

struct X {
    x: u8,
}

// no errors, bounded lifetimes:
trait Trait<'a> {}
fn trait_bound<'a, T: Trait<'a>>(x: &'a u8, y: T) {}
fn trait_bound_where<'a, T>(x: &'a u8, y: T) where T: Trait<'a> {}
fn lt_bound<'a, T: 'a>(x: &'a u8, y: T) {}
fn lt_bound_where<'a, T>(x: &'a u8, y: T) where T : 'a {}

impl X {
    fn self_and_out<'s>(&'s self) -> &'s u8 { &self.x }
    //~^ERROR explicit lifetimes given

    fn self_and_in_out<'s, 't>(&'s self, x: &'t u8) -> &'s u8 { &self.x } // no error, multiple input refs

    fn distinct_self_and_in<'s, 't>(&'s self, x: &'t u8) { }
    //~^ERROR explicit lifetimes given

    fn self_and_same_in<'s>(&'s self, x: &'s u8) { } // no error, same lifetimes on two params
}

struct Foo<'a>(&'a u8);

impl<'a> Foo<'a> {
    fn self_shared_lifetime(&self, _: &'a u8) {} // no error, lifetime 'a not defined in method
    fn self_bound_lifetime<'b: 'a>(&self, _: &'b u8) {} // no error, bounds exist
}

fn main() {
}
