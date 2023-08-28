#![warn(clippy::short_circuit_statement)]
#![allow(clippy::nonminimal_bool)]

fn main() {
    f() && g();
    //~^ ERROR: boolean short circuit operator in statement may be clearer using an explic
    //~| NOTE: `-D clippy::short-circuit-statement` implied by `-D warnings`
    f() || g();
    //~^ ERROR: boolean short circuit operator in statement may be clearer using an explic
    1 == 2 || g();
    //~^ ERROR: boolean short circuit operator in statement may be clearer using an explic
}

fn f() -> bool {
    true
}

fn g() -> bool {
    false
}
