#![warn(clippy::doc_examples_missing_item)]

mod inner {
    /// ```
    /// let _ = 1;
    /// ```
    fn private_copied() {}
    //~^ doc_examples_missing_item

    /// ```
    /// private_own();
    /// ```
    fn private_own() {}
}

#[doc(hidden)]
pub mod hidden_module {
    /// ```
    /// let _ = 1;
    /// ```
    pub fn under_hidden() {}
    //~^ doc_examples_missing_item
}

fn main() {}
