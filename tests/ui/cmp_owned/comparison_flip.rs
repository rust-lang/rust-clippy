use std::fmt::{self, Display};

fn main() {
    let a = Foo;

    //~v cmp_owned
    if a.to_string() != "bar" {
        println!("foo");
    }

    //~v cmp_owned
    if "bar" != a.to_string() {
        println!("foo");
    }
}

struct Foo;

impl Display for Foo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "foo")
    }
}

impl PartialEq<&str> for Foo {
    fn eq(&self, other: &&str) -> bool {
        "foo" == *other
    }
}
