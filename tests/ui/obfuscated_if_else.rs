#![warn(clippy::obfuscated_if_else)]

fn main() {
    true.then_some("a").unwrap_or("b");
    //~^ ERROR: use of `.then_some(..).unwrap_or(..)` can be written more clearly with `if
    //~| NOTE: `-D clippy::obfuscated-if-else` implied by `-D warnings`
}
