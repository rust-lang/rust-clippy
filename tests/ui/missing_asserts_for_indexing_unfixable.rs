#![warn(clippy::missing_asserts_for_indexing)]

fn sum(v: &[u8]) -> u8 {
    v[0] + v[1] + v[2] + v[3] + v[4]
    //~^ missing_asserts_for_indexing
}

fn subslice(v: &[u8]) {
    let _ = v[0];
    //~^ missing_asserts_for_indexing

    let _ = v[1..4];
}

fn variables(v: &[u8]) -> u8 {
    let a = v[0];
    //~^ missing_asserts_for_indexing

    let b = v[1];
    let c = v[2];
    a + b + c
}

fn index_different_slices(v1: &[u8], v2: &[u8]) {
    let _ = v1[0] + v1[12];
    //~^ missing_asserts_for_indexing
    let _ = v2[5] + v2[15];
    //~^ missing_asserts_for_indexing
}

fn index_different_slices2(v1: &[u8], v2: &[u8]) {
    assert!(v1.len() > 12);
    let _ = v1[0] + v1[12];
    let _ = v2[5] + v2[15];
    //~^ missing_asserts_for_indexing
}

struct Foo<'a> {
    v: &'a [u8],
    v2: &'a [u8],
}

fn index_struct_field(f: &Foo<'_>) {
    let _ = f.v[0] + f.v[1];
    //~^ missing_asserts_for_indexing
}

fn index_struct_different_fields(f: &Foo<'_>) {
    // ok, different fields
    let _ = f.v[0] + f.v2[1];
}

fn shadowing() {
    let x: &[i32] = &[1];
    assert!(x.len() > 1);

    let x: &[i32] = &[1];
    let _ = x[0] + x[1];
    //~^ missing_asserts_for_indexing
}

pub fn issue11856(values: &[i32]) -> usize {
    let mut ascending = Vec::new();
    for w in values.windows(2) {
        assert!(w.len() > 1);
        if w[0] < w[1] {
            ascending.push((w[0], w[1]));
        } else {
            ascending.push((w[1], w[0]));
        }
    }
    ascending.len()
}

fn assert_after_indexing(v1: &[u8]) {
    let _ = v1[1] + v1[2];
    //~^ ERROR: indexing into a slice multiple times without an `assert`
    assert!(v1.len() > 2);
}

fn issue14255(v1: &[u8]) {
    assert_ne!(v1.len(), 2);

    let _ = v1[0] + v1[1] + v1[2];
    //~^ missing_asserts_for_indexing
}

// not contiguous from 0: `_` can still be 0
fn non_contiguous(supported: &[u8]) {
    match supported.len() {
        5 => {},
        _ => println!("{} or {}", supported[0], supported[1]),
        //~^ missing_asserts_for_indexing
    }
}

// wildcard only: no length information
#[allow(clippy::match_single_binding)]
fn wildcard_only(supported: &[u8]) {
    match supported.len() {
        _ => println!("{} or {}", supported[0], supported[1]),
        //~^ missing_asserts_for_indexing
    }
}

// guard defeats the exclusion
fn with_guard(supported: &[u8], flag: bool) {
    match supported.len() {
        0 if flag => {},
        1 => {},
        _ => println!("{} or {}", supported[0], supported[1]),
        //~^ missing_asserts_for_indexing
    }
}

// arms only guarantee len > 1, but 2 and 3 are indexed
fn index_too_high(supported: &[u8]) {
    match supported.len() {
        0 => {},
        1 => {},
        _ => println!("{} {}", supported[2], supported[3]),
        //~^ missing_asserts_for_indexing
    }
}

// literal arm pins len to exactly 2, so indexing at 2 and 3 must lint
fn literal_arm_out_of_bounds(supported: &[u8]) {
    match supported.len() {
        0 => {},
        1 => {},
        2 => println!("{} {}", supported[2], supported[3]),
        //~^ missing_asserts_for_indexing
        _ => {},
    }
}

fn main() {}
