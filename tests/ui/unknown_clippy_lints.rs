#![warn(clippy::pedantic)]
// Should suggest lowercase
#![allow(clippy::All)]
//~^ ERROR: unknown lint: `clippy::All`
//~| NOTE: `-D unknown-lints` implied by `-D warnings`
#![warn(clippy::CMP_OWNED)]
//~^ ERROR: unknown lint: `clippy::CMP_OWNED`

// Should suggest similar clippy lint name
#[warn(clippy::if_not_els)]
//~^ ERROR: unknown lint: `clippy::if_not_els`
#[warn(clippy::UNNecsaRy_cAst)]
//~^ ERROR: unknown lint: `clippy::UNNecsaRy_cAst`
#[warn(clippy::useles_transute)]
//~^ ERROR: unknown lint: `clippy::useles_transute`
// Shouldn't suggest rustc lint name(`dead_code`)
#[warn(clippy::dead_cod)]
//~^ ERROR: unknown lint: `clippy::dead_cod`
// Shouldn't suggest removed/deprecated clippy lint name(`unused_collect`)
#[warn(clippy::unused_colle)]
//~^ ERROR: unknown lint: `clippy::unused_colle`
// Shouldn't suggest renamed clippy lint name(`const_static_lifetime`)
#[warn(clippy::const_static_lifetim)]
//~^ ERROR: unknown lint: `clippy::const_static_lifetim`
fn main() {}
