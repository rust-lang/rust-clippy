#![warn(clippy::short_circuit_statement)]
#![allow(clippy::nonminimal_bool)]

fn main() {
    f() && g(); //~ short_circuit_statement
    f() || g(); //~ short_circuit_statement
    1 == 2 || g(); //~ short_circuit_statement
}

fn f() -> bool {
    true
}

fn g() -> bool {
    false
}
