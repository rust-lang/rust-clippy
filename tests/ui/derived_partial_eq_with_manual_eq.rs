#![warn(clippy::derived_partial_eq_with_manual_eq)]

// ok
#[derive(PartialEq, Eq)]
struct BothImplDerived;

// ok
struct BothImplManual {
    value: u8,
}
impl PartialEq for BothImplManual {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for BothImplManual {}

#[derive(PartialEq)]
struct OneImplManual;
impl Eq for OneImplManual {}
//~^^^ derived_partial_eq_with_manual_eq

// it's ok to derive `Eq` and implement `PartialEq` manually
// ok
#[derive(Eq, Debug, Clone, Copy, Ord, PartialOrd)]
struct WithTypeDefinition {
    value: usize,
    value2: usize,
}
impl PartialEq for WithTypeDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.value2 == other.value2
    }
}

#[derive(PartialEq, Debug, Default, PartialOrd, Copy, Clone, Ord)]
struct WithTypeDefinition2 {
    value: usize,
    value2: usize,
}
impl Eq for WithTypeDefinition2 {}
//~^^^^^^ derived_partial_eq_with_manual_eq

#[derive(PartialEq)]
struct WithTypeDefinition3<T> {
    value: T,
}
impl<T> Eq for WithTypeDefinition3<T> where T: PartialEq {}
//~^^^^^ derived_partial_eq_with_manual_eq

#[derive(PartialEq)]
struct WithTypeDefinition4<T> {
    value: T,
}
impl<T: PartialEq> Eq for WithTypeDefinition4<T> {}
//~^^^^^ derived_partial_eq_with_manual_eq

#[derive(PartialEq)]
enum EnumWithManualEq {
    A(u8),
    B(String),
}
impl Eq for EnumWithManualEq {}
//~^^^^^^ derived_partial_eq_with_manual_eq

// ok
#[derive(Eq)]
enum EnumWithDerivedEq {
    A(u8),
    B(String),
}
impl PartialEq for EnumWithDerivedEq {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&EnumWithDerivedEq::A(_), &EnumWithDerivedEq::A(_))
            | (&EnumWithDerivedEq::B(_), &EnumWithDerivedEq::B(_)) => todo!(),

            (&EnumWithDerivedEq::A(_), &EnumWithDerivedEq::B(_))
            | (&EnumWithDerivedEq::B(_), &EnumWithDerivedEq::A(_)) => todo!(),
        }
    }
}

// ok
enum EnumWithDerivedEq2 {
    A(usize),
    B(Vec<String>),
}
impl PartialEq for EnumWithDerivedEq2 {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&EnumWithDerivedEq2::A(_), &EnumWithDerivedEq2::A(_))
            | (&EnumWithDerivedEq2::B(_), &EnumWithDerivedEq2::B(_)) => todo!(),

            (&EnumWithDerivedEq2::A(_), &EnumWithDerivedEq2::B(_))
            | (&EnumWithDerivedEq2::B(_), &EnumWithDerivedEq2::A(_)) => todo!(),
        }
    }
}
impl Eq for EnumWithDerivedEq2 {}

// test the type from macro expansion is correctly detected
macro_rules! mk_type {
    ($name:ident) => {
        #[derive(PartialEq)]
        struct $name(u8);
        impl Eq for $name {}
    };
}
// NOTE: Please don't remove this between `mk_type!` and `macro_rules!`,
// the test will crash when the macro usage near the macro definition.
mk_type!(StructEqFromExpansion);
//~^ derived_partial_eq_with_manual_eq

fn main() {}
