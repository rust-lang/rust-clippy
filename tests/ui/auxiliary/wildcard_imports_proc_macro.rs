extern crate proc_macro;

use proc_macro::{Group, TokenStream, TokenTree};

#[proc_macro_derive(WildcardImport)]
pub fn derive_wildcard_import(input: TokenStream) -> TokenStream {
    let span = input.into_iter().next().unwrap().span();
    let generated = r#"
        fn generated_by_proc_macro() {
            use crate::fn_mod::*;
            foo();
        }
    "#
    .parse()
    .unwrap();

    respan(generated, span)
}

fn respan(stream: TokenStream, span: proc_macro::Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => {
                let mut respanned = Group::new(group.delimiter(), respan(group.stream(), span));
                respanned.set_span(span);
                TokenTree::Group(respanned)
            },
            TokenTree::Ident(mut ident) => {
                ident.set_span(span);
                TokenTree::Ident(ident)
            },
            TokenTree::Punct(mut punct) => {
                punct.set_span(span);
                TokenTree::Punct(punct)
            },
            TokenTree::Literal(mut literal) => {
                literal.set_span(span);
                TokenTree::Literal(literal)
            },
        })
        .collect()
}
