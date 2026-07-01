#![warn(clippy::manual_ilog10)]

fn call(base: u32) {
    let a = 0u32;
    a.ilog(base); // Should not lint because it's not a literal.
}

fn main() {
    let a = 0u32;
    a.ilog(10); //~ manual_ilog10

    // don't lint when macros are involved
    macro_rules! ten {
        () => {
            10
        };
    };

    a.ilog(ten!());
    call(10);
}
