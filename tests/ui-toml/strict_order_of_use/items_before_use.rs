#![warn(clippy::items_before_use)]

mod demo {
    use std::fmt; // OK: at the top

    extern crate core;
    use std::io; //~ items_before_use

    mod submodule {}
    use std::fs; //~ items_before_use

    fn f() {}
    use std::str; //~ items_before_use

    #[cfg(test)]
    mod test_mod {
        use std::collections::HashMap; // ignored (inside cfg block)
        fn test_fn() {}
    }

    #[cfg(feature = "magic")]
    use std::vec::Vec; // ignored (inside cfg block)

    struct Data;
    use std::borrow::Cow; //~ items_before_use
}

fn main() {}
