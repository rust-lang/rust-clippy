// run-rustfix

#![allow(
    unused,
    clippy::no_effect,
    clippy::redundant_closure_call,
    clippy::many_single_char_names,
    clippy::needless_pass_by_value,
    clippy::option_map_unit_fn,
    clippy::trivially_copy_pass_by_ref,
    )]
#![warn(clippy::redundant_closure, clippy::needless_borrow)]

use std::path::PathBuf;

fn main() {
    let a = Some(1u8).map(|a| foo(a));
    meta(|a| foo(a));
    let c = Some(1u8).map(|a| {1+2; foo}(a));
    let d = Some(1u8).map(|a| foo((|b| foo2(b))(a))); //is adjusted?
    all(&[1, 2, 3], &&2, |x, y| below(x, y)); //is adjusted
    unsafe {
        Some(1u8).map(|a| unsafe_fn(a)); // unsafe fn
    }

    // See #815
    let e = Some(1u8).map(|a| divergent(a));
    let e = Some(1u8).map(|a| generic(a));
    let e = Some(1u8).map(generic);
    // See #515
    let a: Option<Box<::std::ops::Deref<Target = [i32]>>> =
        Some(vec![1i32, 2]).map(|v| -> Box<::std::ops::Deref<Target = [i32]>> { Box::new(v) });
}

trait TestTrait {
    fn trait_foo(self) -> bool;
    fn trait_foo_ref(&self) -> bool;
}

struct TestStruct<'a> {
    some_ref: &'a i32,
}

impl<'a> TestStruct<'a> {
    fn foo(self) -> bool {
        false
    }
    unsafe fn foo_unsafe(self) -> bool {
        true
    }
}

impl<'a> TestTrait for TestStruct<'a> {
    fn trait_foo(self) -> bool {
        false
    }
    fn trait_foo_ref(&self) -> bool {
        false
    }
}

impl<'a> std::ops::Deref for TestStruct<'a> {
    type Target = char;
    fn deref(&self) -> &char {
        &'a'
    }
}

fn test_redundant_closures_containing_method_calls() {
    
    // In TestStruct use cases
    {
        let i = 10;
        let _ = Some(TestStruct { some_ref: &i }).map(|a| a.foo());
        let _ = Some(TestStruct { some_ref: &i }).map(TestStruct::foo);
        let _ = Some(TestStruct { some_ref: &i }).map(|a| a.trait_foo());
        let _ = Some(TestStruct { some_ref: &i }).map(|a| a.trait_foo_ref());
        let _ = Some(TestStruct { some_ref: &i }).map(TestTrait::trait_foo);
        let _ = Some(TestStruct { some_ref: &i })
            .as_ref()
            .map(|c| c.to_ascii_uppercase());
        unsafe {
            let _ = Some(TestStruct { some_ref: &i }).map(|a| a.foo_unsafe());
        }
    }

    // In a function that uses TestTrait
    fn test_different_borrow_levels<T>(t: &[&T])
    where T: TestTrait,
    {
        t.iter().filter(|x| x.trait_foo_ref());
        t.iter().map(|x| x.trait_foo_ref());
    }

    // In Vector operations
    {
        let _ = Some(&mut vec![1, 2, 3]).map(|v| v.clear());
        let _ = Some(&mut vec![1, 2, 3]).map(std::vec::Vec::clear);
    }

    // In String and Char operations
    {
        let _ = Some("str").map(|s| s.to_string());
        let _ = Some("str").map(str::to_string);
        let _ = Some('a').map(|s| s.to_uppercase());
        let _ = Some('a').map(char::to_uppercase);
    }

    // In Iterator Maps
    {
        let _: std::vec::Vec<usize> = vec!['a', 'b', 'c'].iter().map(|c| c.len_utf8()).collect();
        let _: std::vec::Vec<char> = vec!['a', 'b', 'c'].iter().map(|c| c.to_ascii_uppercase()).collect();
        let _: std::vec::Vec<char> = vec!['a', 'b', 'c'].iter().map(char::to_ascii_uppercase).collect();
    }

    // In other Cases
    {
        let p = Some(PathBuf::new());
        let _ = p.as_ref().and_then(|s| s.to_str());
    }
}

fn meta<F>(f: F)
where
    F: Fn(u8),
{
    f(1u8)
}

fn foo(_: u8) {}

fn foo2(_: u8) -> u8 {
    1u8
}

fn all<X, F>(x: &[X], y: &X, f: F) -> bool
where
    F: Fn(&X, &X) -> bool,
{
    x.iter().all(|e| f(e, y))
}

fn below(x: &u8, y: &u8) -> bool {
    x < y
}

unsafe fn unsafe_fn(_: u8) {}

fn divergent(_: u8) -> ! {
    unimplemented!()
}

fn generic<T>(_: T) -> u8 {
    0
}
