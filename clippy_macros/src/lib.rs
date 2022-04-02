#![feature(concat_idents)]
#![feature(exact_size_is_empty)]
#![feature(let_else)]

mod sugg;

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn expr_sugg(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as sugg::ExprSugg).0.into()
}
