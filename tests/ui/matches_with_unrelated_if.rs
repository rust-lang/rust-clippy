#![warn(clippy::matches_with_unrelated_if)]
#![expect(clippy::redundant_pattern_matching)]

struct Foo;

impl Foo {
    fn foo(&self) -> Option<i32> {
        todo!()
    }
    fn is_bar(&self) -> bool {
        todo!()
    }
}

macro_rules! wrapper {
    ($e:expr) => {
        matches!($e, Some(_) if true)
    };
}

fn main() {
    let f = Foo;
    let (a, b) = (false, true);

    let _ = matches!(f.foo(), Some(_) if f.is_bar());
    //~^ matches_with_unrelated_if

    // guard needs parens
    let _ = matches!(f.foo(), Some(_) if a || b);
    //~^ matches_with_unrelated_if

    let _ = !matches!(f.foo(), Some(_) if a);
    //~^ matches_with_unrelated_if

    let _ = matches!(f.foo(), Some(_) | None if a);
    //~^ matches_with_unrelated_if

    // `&&` parent is not an `||` operand, so this should still lints
    let _ = matches!(f.foo(), Some(_) if a) && b;
    //~^ matches_with_unrelated_if

    let _ = matches!(f.foo(), Some(x) if x > 0);

    let _ = matches!(f.foo(), Some(_));

    // operands of `||`: see `https://github.com/rust-lang/rust-clippy/pull/16870#issuecomment-4603344307`
    let _ = matches!(f.foo(), Some(_) if a) || matches!(f.foo(), None if b);

    let _ = wrapper!(f.foo());
}
