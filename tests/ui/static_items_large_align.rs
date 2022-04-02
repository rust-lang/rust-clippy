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

struct AlignedPtr(*const Aligned);

unsafe impl Sync for AlignedPtr {}

struct AlignedGeneric<T>(T);

static X: Aligned = Aligned(0);

static X_REF: &Aligned = &Aligned(0);

static X_PTR: AlignedPtr = AlignedPtr(&Aligned(0) as *const _);

static XW: AlignedWrapper = AlignedWrapper { f: 0, g: Aligned(0) };

static ARRAY: [Aligned; 10] = [Aligned(0); 10];

static TUPLE: (u8, (Aligned, u8)) = (0, (Aligned(0), 0));

static XE: AlignedEnum = AlignedEnum::A(Aligned(0));

static XG: AlignedGeneric<Aligned> = AlignedGeneric(Aligned(0));

static XGW: AlignedGeneric<AlignedWrapper> = AlignedGeneric(AlignedWrapper { f: 0, g: Aligned(0) });

fn main() {
    let x = Aligned(0);
    println!("{:#x}", Box::into_raw(Box::new(Aligned(0))) as usize);
}

////////////////////////////////////////////////////////////////
/////////////// below is a more involved example ///////////////
////////////////////////////////////////////////////////////////

#[repr(align(0x100000))]
struct AlignedA(u8);

#[repr(align(0x100000))]
struct AlignedB(u8);

struct FnPtr<T>(fn() -> Box<T>);

struct AG<T>(T);

type AGAlias<T> = AG<T>;

struct AGWithArgs<A, B> {
    not_aligned: FnPtr<A>,
    aligned: AGAlias<B>,
}

static XG_ARGS: AGWithArgs<AlignedA, AlignedB> = AGWithArgs {
    not_aligned: FnPtr(box_aligned),
    aligned: AG(AlignedB(0)),
};

fn box_aligned() -> Box<AlignedA> {
    Box::new(AlignedA(0))
}
