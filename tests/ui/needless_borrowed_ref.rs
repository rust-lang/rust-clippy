#![warn(clippy::needless_borrowed_reference)]
#![allow(
    unused,
    irrefutable_let_patterns,
    non_shorthand_field_patterns,
    clippy::needless_borrow,
    clippy::needless_if
)]

fn main() {}

struct Struct {
    a: usize,
    b: usize,
    c: usize,
}

struct TupleStruct(u8, u8, u8);

fn should_lint(
    array: [u8; 4],
    slice: &[u8],
    slice_of_refs: &[&u8],
    vec: Vec<u8>,
    tuple: (u8, u8, u8),
    tuple_struct: TupleStruct,
    s: Struct,
) {
    let mut v = Vec::<String>::new();
    let _ = v.iter_mut().filter(|&ref a| a.is_empty());
    //~^ ERROR: this pattern takes a reference on something that is being dereferenced
    //~| NOTE: `-D clippy::needless-borrowed-reference` implied by `-D warnings`

    let var = 3;
    let thingy = Some(&var);
    if let Some(&ref v) = thingy {}
    //~^ ERROR: this pattern takes a reference on something that is being dereferenced

    if let &[&ref a, ref b] = slice_of_refs {}
    //~^ ERROR: this pattern takes a reference on something that is being dereferenced

    let &[ref a, ..] = &array;
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference
    let &[ref a, ref b, ..] = &array;
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference

    if let &[ref a, ref b] = slice {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference
    if let &[ref a, ref b] = &vec[..] {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference

    if let &[ref a, ref b, ..] = slice {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference
    if let &[ref a, .., ref b] = slice {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference
    if let &[.., ref a, ref b] = slice {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference

    if let &[ref a, _] = slice {}
    //~^ ERROR: dereferencing a slice pattern where every element takes a reference

    if let &(ref a, ref b, ref c) = &tuple {}
    //~^ ERROR: dereferencing a tuple pattern where every element takes a reference
    if let &(ref a, _, ref c) = &tuple {}
    //~^ ERROR: dereferencing a tuple pattern where every element takes a reference
    if let &(ref a, ..) = &tuple {}
    //~^ ERROR: dereferencing a tuple pattern where every element takes a reference

    if let &TupleStruct(ref a, ..) = &tuple_struct {}
    //~^ ERROR: dereferencing a tuple pattern where every element takes a reference

    if let &Struct {
    //~^ ERROR: dereferencing a struct pattern where every field's pattern takes a referen
        ref a,
        b: ref b,
        c: ref renamed,
    } = &s
    {}

    if let &Struct { ref a, b: _, .. } = &s {}
    //~^ ERROR: dereferencing a struct pattern where every field's pattern takes a referen
}

fn should_not_lint(
    array: [u8; 4],
    slice: &[u8],
    slice_of_refs: &[&u8],
    vec: Vec<u8>,
    tuple: (u8, u8, u8),
    tuple_struct: TupleStruct,
    s: Struct,
) {
    if let [ref a] = slice {}
    if let &[ref a, b] = slice {}
    if let &[ref a, .., b] = slice {}

    if let &(ref a, b, ..) = &tuple {}
    if let &TupleStruct(ref a, b, ..) = &tuple_struct {}
    if let &Struct { ref a, b, .. } = &s {}

    // must not be removed as variables must be bound consistently across | patterns
    if let (&[ref a], _) | ([], ref a) = (slice_of_refs, &1u8) {}

    // the `&`s here technically could be removed, but it'd be noisy and without a `ref` doesn't match
    // the lint name
    if let &[] = slice {}
    if let &[_] = slice {}
    if let &[..] = slice {}
    if let &(..) = &tuple {}
    if let &TupleStruct(..) = &tuple_struct {}
    if let &Struct { .. } = &s {}

    let mut var2 = 5;
    let thingy2 = Some(&mut var2);
    if let Some(&mut ref mut v) = thingy2 {
        //          ^ should **not** be linted
        // v is borrowed as mutable.
        *v = 10;
    }
    if let Some(&mut ref v) = thingy2 {
        //          ^ should **not** be linted
        // here, v is borrowed as immutable.
        // can't do that:
        //*v = 15;
    }
}

enum Animal {
    Cat(u64),
    Dog(u64),
}

fn foo(a: &Animal, b: &Animal) {
    match (a, b) {
        // lifetime mismatch error if there is no '&ref' before `feature(nll)` stabilization in 1.63
        (&Animal::Cat(v), &ref k) | (&ref k, &Animal::Cat(v)) => (),
        //                  ^    and   ^ should **not** be linted
        (Animal::Dog(a), &Animal::Dog(_)) => (),
    }
}
