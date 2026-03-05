#![warn(clippy::unused_format_precision)]
#![allow(clippy::zero_ptr, clippy::manual_dangling_ptr)]

fn main() {
    let v = 42_u8;
    println!("{:.1}", 42_u8); //~ ERROR: precision has no effect for type `u8`
    println!("{:.1?}", 42_u8); //~ ERROR: precision has no effect for type `u8`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `u8`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `u8`

    let prec = 1;
    println!("{:.prec$}", 42_u8); //~ ERROR: precision has no effect for type `u8`
    println!("{0:.1$}", 42_u8, prec); //~ ERROR: precision has no effect for type `u8`
    println!("{:.*}", prec, 42_u8); //~ ERROR: precision has no effect for type `u8`
    println!("{:.prec$?}", 42_u8); //~ ERROR: precision has no effect for type `u8`
    println!("{v:.prec$}"); //~ ERROR: precision has no effect for type `u8`
    println!("{:.prec$}", &v); //~ ERROR: precision has no effect for type `u8`

    let v = 42_i8;
    println!("{:.1}", 42_i8); //~ ERROR: precision has no effect for type `i8`
    println!("{:.1?}", 42_i8); //~ ERROR: precision has no effect for type `i8`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `i8`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `i8`

    let v = 42_u16;
    println!("{:.1}", 42_u16); //~ ERROR: precision has no effect for type `u16`
    println!("{:.1?}", 42_u16); //~ ERROR: precision has no effect for type `u16`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `u16`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `u16`

    let v = 42_i16;
    println!("{:.1}", 42_i16); //~ ERROR: precision has no effect for type `i16`
    println!("{:.1?}", 42_i16); //~ ERROR: precision has no effect for type `i16`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `i16`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `i16`

    let v = 42_u32;
    println!("{:.1}", 42_u32); //~ ERROR: precision has no effect for type `u32`
    println!("{:.1?}", 42_u32); //~ ERROR: precision has no effect for type `u32`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `u32`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `u32`

    let v = 42_i32;
    println!("{:.1}", 42_i32); //~ ERROR: precision has no effect for type `i32`
    println!("{:.1?}", 42_i32); //~ ERROR: precision has no effect for type `i32`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `i32`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `i32`

    let v = 42_u64;
    println!("{:.1}", 42_u64); //~ ERROR: precision has no effect for type `u64`
    println!("{:.1?}", 42_u64); //~ ERROR: precision has no effect for type `u64`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `u64`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `u64`

    let v = 42_i64;
    println!("{:.1}", 42_i64); //~ ERROR: precision has no effect for type `i64`
    println!("{:.1?}", 42_i64); //~ ERROR: precision has no effect for type `i64`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `i64`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `i64`

    let v = 42_usize;
    println!("{:.1}", 42_usize); //~ ERROR: precision has no effect for type `usize`
    println!("{:.1?}", 42_usize); //~ ERROR: precision has no effect for type `usize`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `usize`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `usize`

    let v = 42_isize;
    println!("{:.1}", 42_isize); //~ ERROR: precision has no effect for type `isize`
    println!("{:.1?}", 42_isize); //~ ERROR: precision has no effect for type `isize`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `isize`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `isize`

    let v = 'x';
    println!("{:.1}", 'x'); //~ ERROR: precision has no effect for type `char`
    println!("{:.1?}", 'x'); //~ ERROR: precision has no effect for type `char`
    println!("{v:.1}"); //~ ERROR: precision has no effect for type `char`
    println!("{:.1}", &v); //~ ERROR: precision has no effect for type `char`

    let v = 0x123456789 as *const usize;
    println!("{:.1p}", 0x123456789 as *const u8); //~ ERROR: precision has no effect for type `*const u8`
    println!("{v:.1p}"); //~ ERROR: precision has no effect for type `*const usize`

    fn dummy() {}
    let v = dummy as fn();
    println!("{:.1p}", dummy as fn()); //~ ERROR: precision has no effect for type `fn()`
    println!("{v:.1p}"); //~ ERROR: precision has no effect for type `fn()`

    // Not linted - precision used for truncation
    println!("{:.1}", "hello");
    println!("{:.1}", &"hello");

    let v = true;
    println!("{:.1}", true);
    println!("{:.1?}", true);
    println!("{v:.1}");
    println!("{:.1}", &v);

    // Not linted: floats (precision used)
    println!("{:.1}", 1.0f32);
    println!("{:.1}", 1.0f64);

    // Not linted: no precision specified
    println!("{}", 1u8);
    println!("{:?}", 42);

    // Not linted: custom types (ignored by design)
    struct S;
    impl std::fmt::Display for S {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ABC")
        }
    }
    println!("{:.1}", S);

    // Generics: not linted, since we don't know the type
    fn generic<T: std::fmt::Display>(x: T) {
        println!("{:.1}", x);
    }
}
