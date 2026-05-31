#![warn(clippy::too_many_fields)]

macro_rules! foo {
    () => {
        struct MacroFoo {
            a: u8,
            b: u8,
            c: u8,
            d: u8,
            e: u8,
            f: u8,
            g: u8,
            h: u8,
            i: u8,
            j: u8,
            k: u8,
        }
    };
}

foo!();

struct TenFields {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    g: u8,
    h: u8,
    i: u8,
    j: u8,
}

struct ElevenFields {
    //~^ too_many_fields
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    g: u8,
    h: u8,
    i: u8,
    j: u8,
    k: u8,
}

struct TupleFields(
    //~^ too_many_fields
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
);

#[repr(C)]
struct ReprFields {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    g: u8,
    h: u8,
    i: u8,
    j: u8,
    k: u8,
}

fn main() {
    struct LocalFields {
        //~^ too_many_fields
        a: u8,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        f: u8,
        g: u8,
        h: u8,
        i: u8,
        j: u8,
        k: u8,
    }
}
