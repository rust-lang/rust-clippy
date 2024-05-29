//@no-rustfix
#![allow(unused)]
#![allow(dropping_copy_types)]
#![allow(dropping_references)]
#![warn(clippy::unnecessary_indexing)]

fn c(x: i32) -> i32 {
    println!("{x}");
    10
}

struct Struct;
impl Struct {
    pub fn a(x: i32) -> i32 {
        println!("{x}");
        10
    }
}

fn main() {
    // lint on vecs with a call
    let a: Vec<i32> = vec![1];
    if !a.is_empty() {
        let b = c(a[0]);
    }

    // lint on vecs with a method call
    let a: Vec<i32> = vec![1];
    if !a.is_empty() {
        let b = Struct::a(a[0]);
    }

    // lint on arrays with a call
    let a: &[i32] = &[1];
    if !a.is_empty() {
        let b = c(a[0]);
    }

    // lint on arrays with a method call
    let a: &[i32] = &[1];
    if !a.is_empty() {
        let b = Struct::a(a[0]);
    }

    // lint on vecs with a local access
    let a: Vec<i32> = vec![1];
    if !a.is_empty() {
        let b = a[0];
    }

    // lint on arrays with a local access
    let a: &[i32] = &[1];
    if !a.is_empty() {
        let b = a[0];
    }

    // lint when access is not first line
    let a: &[i32] = &[1];
    if !a.is_empty() {
        dbg!(a);
        let b = a[0];
    }

    // lint on multiple accesses/locals
    let a: &[i32] = &[1];
    if !a.is_empty() {
        dbg!(a);
        let b = a[0];
        let c = a[0];
        drop(a[0]);
    }

    // lint on multiple accesses
    let a: &[i32] = &[1];
    if !a.is_empty() {
        dbg!(a);
        drop(a[0]);
        drop(a[0]);
    }

    // dont lint when not accessing [0]
    let a: &[i32] = &[1, 2];
    if !a.is_empty() {
        let b = a[1];
    }

    // dont lint when access is dynamic
    const T: usize = 0;

    let a: &[i32] = &[1];
    if !a.is_empty() {
        let b = a[T];
    }

    // dont lint without unary
    let a: &[i32] = &[1];
    if a.is_empty() {
        let b = a[0];
    }

    // dont lint if we have mutable reference
    let mut a: &[i32] = &[1];
    if !a.is_empty() {
        drop(&mut a);
        let b = a[0];
    }

    // dont lint if we have a mutable reference, even if the mutable reference occurs after what we are
    // linting against
    let mut a: &[i32] = &[1];
    if !a.is_empty() {
        let b = a[0];
        drop(&mut a);
    }
}
