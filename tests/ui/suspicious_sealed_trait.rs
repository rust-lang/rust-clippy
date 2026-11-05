//@ compile-flags: --crate-type=lib

#![warn(clippy::suspicious_sealed_trait)]
#![allow(dead_code)]

mod sealed {
    pub trait Sealed {}

    pub trait SealedGeneric<T> {}
}

pub trait Suspicious<T>: sealed::Sealed {}
//~^ suspicious_sealed_trait

pub trait CorrectlySealedGeneric<T>: sealed::SealedGeneric<T> {}

pub trait CorrectlySealed: sealed::Sealed {}
pub trait CorrectlySealedConst<const N: usize>: sealed::Sealed {}
pub trait CorrectlySealedLt<'a>: sealed::Sealed {}

pub mod public_module {
    pub trait NotSealed {}
}
pub trait UnsealedTrait<T>: public_module::NotSealed {}
