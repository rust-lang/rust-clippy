#![warn(clippy::undocumented_may_panic_call)]
#![allow(clippy::short_circuit_statement)]
#![allow(clippy::manual_is_multiple_of)]

#[clippy::may_panic]
pub fn dangerous(n: usize) -> bool {
    if n % 2 == 0 {
        panic!()
    }
    true
}

fn main() {
    let _ = dangerous(1);
    //~^ undocumented_may_panic_call

    // Panic: This is safe, 1 is an odd number
    let _ = dangerous(1);
}

#[rustfmt::skip]
fn deeper() {
    let v: Vec<usize> = vec![];
    !v.is_empty()
        && dangerous(1);
    //~^ undocumented_may_panic_call
    !v.is_empty()
    // Panic: This is safe, 1 is an odd number
        && dangerous(1);
}

struct MyStruct {
    value: usize,
}

impl MyStruct {
    #[clippy::may_panic]
    fn panic_method(&self) -> u32 {
        self.value.ilog2()
    }
}

fn test_struct_methods() {
    let s = MyStruct { value: 42 };

    let _ = s.panic_method();
    //~^ undocumented_may_panic_call

    // Panic: value is non-zero, so we are safe!
    let _ = s.panic_method();
}

trait MyTrait {
    #[clippy::may_panic]
    fn trait_panic_method(&self) -> u32;
}

struct TraitImpl {
    data: usize,
}

impl MyTrait for TraitImpl {
    fn trait_panic_method(&self) -> u32 {
        self.data.ilog2()
    }
}

fn test_trait_methods() {
    let t = TraitImpl { data: 10 };

    let _ = t.trait_panic_method();
    //~^ undocumented_may_panic_call

    // Panic: t.data is non-zero
    // We are safe!
    let _ = t.trait_panic_method();
}

fn test_trait_object(t: &dyn MyTrait) {
    // Should lint: no comment
    let _ = t.trait_panic_method();
    //~^ undocumented_may_panic_call

    // Panic: This is safe, just trust me.
    let _ = t.trait_panic_method();
}
