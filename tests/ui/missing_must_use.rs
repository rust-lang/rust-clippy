#![feature(trait_alias)]
#![warn(clippy::missing_must_use)]

pub struct PubStructUnitNoMustUse;
//~^ missing_must_use
struct PrivStructUnitNoMustUse;
//~^ missing_must_use

pub struct PubTupleStructNoMustUse(u8);
//~^ missing_must_use
struct PrivTupleStructNoMustUse(u8);
//~^ missing_must_use

pub struct PubStructNoMustUse {
    //~^ missing_must_use
    pub field: u8,
}
struct PrivStructNoMustUse {
    //~^ missing_must_use
    pub field: u8,
}

pub enum PubEnumNoMustUse {
    //~^ missing_must_use
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}
enum PrivEnumNoMustUse {
    //~^ missing_must_use
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}

pub union PubUnionNoMustUse {
    //~^ missing_must_use
    f1: u8,
    f2: u16,
}
union PrivUnionNoMustUse {
    //~^ missing_must_use
    f1: u8,
    f2: u16,
}

#[must_use]
pub struct PubStructUnitMustUse;
#[must_use]
struct PrivStructUnitMustUse;

#[must_use]
pub struct PubStructTupleMustUse(u8);
#[must_use]
struct PrivStructTupleMustUse(u8);

#[must_use]
pub struct PubStructMustUse {
    pub field: u8,
}
#[must_use]
struct PrivStructMustUse {
    pub field: u8,
}

#[must_use]
pub enum PubEnumMustUse {
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}
#[must_use]
enum PrivEnumMustUse {
    Unit,
    Tuple(u8),
    Struct { field: u8 },
}

#[must_use]
pub union PubUnionMustUse {
    f1: u8,
    f2: u16,
}
#[must_use]
union PrivUnionMustUse {
    f1: u8,
    f2: u16,
}

const IGNORED_CONST: PubEnumMustUse = PubEnumMustUse::Unit;
static IGNORED_STATIC: PubStructUnitMustUse = PubStructUnitMustUse;

mod ignored_mod {}
use ignored_mod as _;

unsafe extern "C" {
    fn ignored_foreign_fn();
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
std::arch::global_asm!("");

type IgnoredTypeAlias = PubStructUnitMustUse;
trait IgnoredTraitAliasBase {}
trait IgnoredTraitAlias = IgnoredTraitAliasBase;

macro_rules! ignored_macro {
    () => {};
}

extern crate core as ignored_core;
use ignored_core::fmt::Debug as _;

pub fn pub_fn_no_must_use(_: PubStructUnitMustUse) -> u8 {
    //~^ missing_must_use
    0
}
fn priv_fn_no_must_use(_: PubEnumMustUse) -> u8 {
    //~^ missing_must_use
    0
}

#[must_use]
pub fn pub_fn_must_use(_: PubStructUnitMustUse) -> u8 {
    0
}
#[must_use]
fn priv_fn_must_use(_: PubEnumMustUse) -> u8 {
    0
}

pub trait PubTraitNoMustUse {
    type Assoc;
    const VALUE: u8;

    fn pub_trait_fn_no_must_use(_: PubStructUnitMustUse) -> u8;
    //~^ missing_must_use
}
trait PrivTraitNoMustUse {
    type Assoc;
    const VALUE: u8;

    fn priv_trait_fn_no_must_use(_: PubEnumMustUse) -> u8;
    //~^ missing_must_use
}
trait TraitMustUse {
    type Assoc;
    const VALUE: u8;

    #[must_use]
    fn trait_fn_must_use(_: PubStructUnitMustUse) -> u8;
}

impl PubStructUnitMustUse {
    const IGNORED_IMPL_CONST: u8 = 0;

    pub fn pub_impl_fn_no_must_use(_: PubStructUnitMustUse) -> u8 {
        //~^ missing_must_use
        0
    }

    fn priv_impl_fn_no_must_use(_: PubEnumMustUse) -> u8 {
        //~^ missing_must_use
        0
    }

    #[must_use]
    pub fn pub_impl_fn_must_use(_: PubStructUnitMustUse) -> u8 {
        0
    }
}

trait ImplTrait {
    type Assoc;
    const VALUE: u8;

    #[must_use]
    fn implemented_fn(_: PubStructUnitMustUse) -> u8;
}
impl ImplTrait for PubStructUnitMustUse {
    type Assoc = PubEnumMustUse;
    const VALUE: u8 = 0;

    fn implemented_fn(_: PubStructUnitMustUse) -> u8 {
        //~^ missing_must_use
        0
    }
}

// Ignore entry point function
fn main() {}
