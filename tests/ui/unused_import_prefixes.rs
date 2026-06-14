#![warn(clippy::unused_import_prefixes)]

mod mod_one {
    pub struct StructOne;
}

mod parent {
    mod child {
        use crate::parent::child::deep::DeepStruct;
        //~^ unused_import_prefixes

        // the `crate::` prefix is needed
        use crate::mod_one::StructOne;

        use crate::parent::child::deep::{ItemA, ItemB};
        //~^ unused_import_prefixes

        // glob imports
        use crate::parent::child::deep::*;
        //~^ unused_import_prefixes

        pub mod deep {
            pub struct DeepStruct;
            pub struct ItemA;
            pub struct ItemB;
        }
    }
}

// do not lint imports inside macro declarations
macro_rules! generate_import {
    () => {
        use crate::mod_one::StructOne;
    };
}

fn main() {
    generate_import!();
}
