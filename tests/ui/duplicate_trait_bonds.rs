#![allow(unused)]
#![warn(clippy::duplicate_trait_bonds)]

struct SomeStruct<T: Clone + Clone> {
    //~^ duplicate_trait_bonds
    value: T,
}

trait ExampleTrait: Sync + Sync {}
//~^ duplicate_trait_bonds

impl<T: Send + Sync + Clone + Sync> ExampleTrait for SomeStruct<T>
//~^ duplicate_trait_bonds
where
    SomeStruct<T>: Copy + Copy + Clone + Copy
{
}
//~^^^ duplicate_trait_bonds

fn func_with_dup_bond<T: std::fmt::Debug + Clone + std::fmt::Debug + Clone + Clone>(_s: SomeStruct<T>) {}
//~^ duplicate_trait_bonds
//~^^ duplicate_trait_bonds

fn main() {
    let s = SomeStruct { value: 42usize };
    func_with_dup_bond(s);
}
