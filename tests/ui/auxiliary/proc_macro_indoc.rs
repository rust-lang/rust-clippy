// compile-flags: --emit=link
// no-prefer-dynamic

#![crate_type = "proc-macro"]

extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree};
use std::str::FromStr;

/// This code was adapted from the indoc crate
#[proc_macro]
pub fn simple_indoc(input: TokenStream) -> TokenStream {
    let token = input.into_iter().next().unwrap();
    let new_token = TokenStream::from_str(&token.to_string())
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    if let TokenTree::Literal(mut lit) = new_token {
        lit.set_span(token.span());
        return TokenStream::from(TokenTree::Literal(lit));
    }
    panic!();
}
