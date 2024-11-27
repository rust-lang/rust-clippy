#![warn(clippy::inconsistent_struct_constructor)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::unnecessary_operation)]
#![allow(clippy::no_effect)]

#[derive(Default)]
struct Foo {
    x: i32,
    y: i32,
    z: i32,
}

fn main() {
    let x = 1;
    let y = 1;
    let z = 1;

    Foo { y, x, z: z };

    Foo {
        z: z,
        x,
        ..Default::default()
    };
}

// https://github.com/rust-lang/rust-clippy/pull/13737#discussion_r1859261645
mod field_attributes {
    struct HirId;
    struct BodyVisitor {
        macro_unsafe_blocks: Vec<HirId>,
        expn_depth: u32,
    }
    fn check_body(condition: bool) {
        BodyVisitor {
            #[expect(clippy::bool_to_int_with_if)] // obfuscates the meaning
            expn_depth: if condition { 1 } else { 0 },
            macro_unsafe_blocks: Vec::new(),
        };
    }
}
