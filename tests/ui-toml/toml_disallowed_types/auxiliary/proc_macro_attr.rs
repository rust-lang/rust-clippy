extern crate proc_macro;
extern crate quote;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn use_std_hash_map(_args: TokenStream, input: TokenStream) -> TokenStream {
    TokenStream::from_iter([
        input,
        quote!(
            pub fn new_function() {
                let _ = std::collections::HashMap::<i32, i32>::default();
            }
        )
        .into(),
    ])
}
