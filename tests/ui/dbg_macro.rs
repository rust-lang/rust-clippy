// compile-flags: --test
#![warn(clippy::dbg_macro)]

fn foo(n: u32) -> u32 {
    if let Some(n) = dbg!(n.checked_sub(4)) { n } else { n }
}

fn factorial(n: u32) -> u32 {
    if dbg!(n <= 1) {
        dbg!(1)
    } else {
        dbg!(n * factorial(n - 1))
    }
}

fn main() {
    dbg!(42);
    dbg!(dbg!(dbg!(42)));
    foo(3) + dbg!(factorial(4));
    dbg!(1, 2, dbg!(3, 4));
    dbg!(1, 2, 3, 4, 5);
}

mod issue7274 {
    trait Thing<'b> {
        fn foo(&self);
    }

    macro_rules! define_thing {
        ($thing:ident, $body:expr) => {
            impl<'a> Thing<'a> for $thing {
                fn foo<'b>(&self) {
                    $body
                }
            }
        };
    }

    struct MyThing;
    define_thing!(MyThing, {
        dbg!(2);
    });
}

#[test]
pub fn issue8481() {
    dbg!(1);
}

#[cfg(test)]
fn foo2() {
    dbg!(1);
}

#[cfg(test)]
mod mod1 {
    fn func() {
        dbg!(1);
    }
}
