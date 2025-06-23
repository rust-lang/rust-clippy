#![warn(clippy::raw_pointers_to_reference)]

struct S;

fn single(s: &S) -> usize {
    (&raw const s).addr() //~ raw_pointers_to_reference
}
fn double(s: &&S) -> usize {
    (&raw const s).addr() //~ raw_pointers_to_reference
}
fn mutable(s: &mut S) -> usize {
    (&raw const s).addr() //~ raw_pointers_to_reference
}
fn single_deref(s: &mut &S) -> usize {
    (&raw const *s).addr() //~ raw_pointers_to_reference
}
fn deref(s: &S) -> usize {
    (&raw const *s).addr()
}
fn r#unsized(s: &str) -> usize {
    (&raw const s).addr()
}
fn double_unsized(s: &&str) -> usize {
    (&raw const s).addr() //~ raw_pointers_to_reference
}
fn double_unsized_deref(s: &&str) -> usize {
    (&raw const *s).addr()
}

fn main() {}
