// The example is adapted from https://github.com/rust-lang/rust/issues/70022

#![warn(clippy::static_items_large_align)]

#[repr(align(0x100000))]
#[derive(Clone, Copy)]
struct Aligned(u8);

struct AlignedWrapper {
    f: u8,
    g: Aligned,
}

enum AlignedEnum {
    A(Aligned),
}

struct AlignedGeneric<T>(T);

static X: Aligned = Aligned(0);

static X_REF: &Aligned = &Aligned(0);

static ARRAY: [Aligned; 10] = [Aligned(0); 10];

static TUPLE: (u8, Aligned) = (0, Aligned(0));

static XW: AlignedWrapper = AlignedWrapper { f: 0, g: Aligned(0) };

static XE: AlignedEnum = AlignedEnum::A(Aligned(0));

static XG: AlignedGeneric<Aligned> = AlignedGeneric(Aligned(0));

fn main() {
    let x = Aligned(0);
    println!("{:#x}", Box::into_raw(Box::new(Aligned(0))) as usize);
}
