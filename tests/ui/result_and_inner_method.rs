#![warn(clippy::result_and_inner_method)]
#![allow(unused)]

#[derive(Clone, Copy)]
struct Flags(u8);

impl Flags {
    fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

fn flags(x: u8) -> Result<Flags, ()> {
    Ok(Flags(x))
}

fn should_lint() {
    let _ = flags(0b0001).and(flags(0b1111));
    //~^ result_and_inner_method
}

fn no_lint_no_inner_and() {
    let a: Result<u8, ()> = Ok(1);
    let b: Result<u8, ()> = Ok(2);
    // u8 has no inherent `and` method — should not lint
    let _ = a.and(b);
}

fn no_lint_already_qualified() {
    // Calling Result::and with qualified syntax — no lint
    let _ = Result::and(flags(0b0001), flags(0b1111));
}

fn main() {}
