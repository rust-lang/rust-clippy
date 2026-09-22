//@check-pass
#![warn(clippy::doc_examples_missing_item)]

extern crate proc_macro;

use proc_macro::TokenStream;

/// ```
/// let _ = 1;
/// ```
#[proc_macro_derive(MyDerivedTrait)]
pub fn myderive(t: TokenStream) -> TokenStream {
    t
}
