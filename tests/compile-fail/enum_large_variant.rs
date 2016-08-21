#![feature(plugin)]
#![plugin(clippy)]

#![allow(dead_code)]
#![allow(unused_variables)]
#![deny(enum_large_variant)]

enum LargeEnum {
    A(i32),
    B([i32; 8000]), //~ ERROR large enum variant found on variant `B`
}

enum AnotherLargeEnum {
    VariantOk(i32, u32),
    ContainingLargeEnum(LargeEnum), //~ ERROR large enum variant found on variant `ContainingLargeEnum`
    ContainingMoreThanOneField(i32, [i32; 8000], [i32; 9500]), //~ ERROR large enum variant found on variant `ContainingMoreThanOneField`
    VoidVariant,
    StructLikeLittle { x: i32, y: i32 },
    StructLikeLarge { x: [i32; 8000], y: i32 }, //~ ERROR large enum variant found on variant `StructLikeLarge`
}

fn main() {

}
