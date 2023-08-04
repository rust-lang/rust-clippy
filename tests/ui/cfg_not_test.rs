#![allow(unused)]
#![warn(clippy::cfg_not_test)]

fn important_check() {}

fn main() {
    // Statement
    #[cfg(not(test))]
    let answer = 42;

    // Expression
    #[cfg(not(test))]
    important_check();
}

// Deeply nested `not(test)`
#[cfg(not(test))]
fn foo() {}
#[cfg(all(debug_assertions, not(test)))]
fn bar() {}
#[cfg(not(any(not(debug_assertions), test)))]
fn baz() {}

#[cfg(test)]
mod tests {}
