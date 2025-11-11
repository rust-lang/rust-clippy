#![warn(clippy::derive_trait_ordering)]

#[derive(Copy, Clone, Debug, Eq, PartialEq)]  // should be okay (already ordered)
struct Ordered;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]  // should be okay (already ordered)
struct Ordered2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]  // should trigger lint (Eq should come before PartialEq)
struct Unordered1;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]  // should trigger lint (Copy should come first)
struct Unordered2;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]  // should trigger lint (Clone, Copy should come first)
struct Unordered3;

#[derive(Debug)]  // single item should be okay
struct Single;

// Multiple derives should not trigger this lint
#[derive(Clone)]
#[derive(Debug, Copy)]
struct MultipleAttributes;

// Edge case with non-alphabetical but single derive
#[derive(Serialize)]  // should be okay
struct CustomDerive;

// Test enum
#[derive(Copy, Clone, Debug, PartialEq, Eq)]  // should trigger lint
enum UnorderedEnum {
    A,
    B,
}

// Test union
#[derive(Copy, Clone, Debug)]  // should trigger lint
union UnorderedUnion {
    a: i32,
    b: f32,
}

// Test that derive ordering ignores case sensitivity issues in sorting
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]  // should trigger lint
struct WithOrd;

fn main() {
    println!("Hello, world!");
}