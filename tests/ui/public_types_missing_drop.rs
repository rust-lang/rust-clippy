#![warn(clippy::public_types_missing_drop)]

struct PrivateType;

// No drop glue anywhere: adding `Drop` later would be a breaking change -> lint fires.
pub struct PublicType(PrivateType);
//~^ public_types_missing_drop

// Has drop glue via a field (`String`): a future `Drop` impl changes nothing observable
// about its drop semantics -> no lint. This is the case `needs_drop` catches but the old
// `has_drop` (explicit-impl-only) check would have wrongly flagged.
pub struct HasDropGlueField(String);

// Explicit `Drop` impl -> already has drop glue -> no lint.
pub struct HasExplicitDrop(PrivateType);

impl Drop for HasExplicitDrop {
    fn drop(&mut self) {}
}

// Enums follow the same rule.
pub enum PublicEnumNoGlue {
    //~^ public_types_missing_drop
    A,
    B(u32),
}

pub enum PublicEnumWithGlue {
    A,
    B(String),
}

// `Copy` types cannot implement `Drop` (E0184), so the suggested fix would not compile, and
// they can never gain drop glue -> no lint, even though they have none currently.
#[derive(Clone, Copy)]
pub struct CopyStruct {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
pub enum CopyEnum {
    A,
    B,
}

// Not exported -> never linted, regardless of drop glue.
struct PrivateNoGlue(PrivateType);

fn main() {}
