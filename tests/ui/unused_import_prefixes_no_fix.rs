//@no-rustfix

#![warn(clippy::unused_import_prefixes)]

mod parent {
    mod child {
        #[rustfmt::skip]
        use crate::parent::{child::deep::ItemA, child::deep::ItemB};
        //~^ unused_import_prefixes

        pub mod deep {
            pub struct ItemA;
            pub struct ItemB;
        }
    }
}
