#![warn(clippy::unnecessary_std_linkage)]
#![crate_type = "lib"]

//~v unnecessary_std_linkage
use core::fmt;

pub struct Foo(pub &'static str);

impl fmt::Debug for Foo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0, fmt)
    }
}
