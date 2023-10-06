#![allow(unused)]
#![warn(clippy::empty_docs)]

pub mod outer_module {

    //!

    //! valid doc comment

    //!!

    //!! valid doc comment

    ///

    /// valid doc comment

    /**
     *
     */

    /**
     * valid block doc comment
     */

    pub mod inner_module {}
}
