//@compile-flags: --crate-name test
//@check-pass
//@no-rustfix

#![warn(clippy::std_wildcard_imports)]

mod crate_items {
    pub fn from_crate() {}
}

mod self_items {
    pub fn from_self() {}
}

use self::self_items::*;
use crate::crate_items::*;

fn main() {
    from_crate();
    from_self();
}
