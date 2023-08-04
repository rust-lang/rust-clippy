#![allow(unused)]
#![warn(clippy::cfg_not_test)]

#[cfg(not(test))]
fn foo() {}

#[cfg(all(debug_assertions, not(test)))]
fn bar() {}

#[cfg(not(any(not(debug_assertions), test)))]
fn baz() {}

fn main() {
    #[cfg(all(debug_assertions, not(test)))]
    let answer = 42;
}

#[cfg(test)]
mod tests {}
