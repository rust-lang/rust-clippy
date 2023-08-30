#![warn(clippy::comparison_to_empty)]
#![allow(clippy::borrow_deref_ref, clippy::needless_if, clippy::useless_vec)]
#![feature(let_chains)]

fn main() {
    // Disallow comparisons to empty
    let s = String::new();
    let _ = s == "";
    //~^ ERROR: comparison to empty slice
    //~| NOTE: `-D clippy::comparison-to-empty` implied by `-D warnings`
    let _ = s != "";
    //~^ ERROR: comparison to empty slice

    let v = vec![0];
    let _ = v == [];
    //~^ ERROR: comparison to empty slice
    let _ = v != [];
    //~^ ERROR: comparison to empty slice
    if let [] = &*v {}
    //~^ ERROR: comparison to empty slice using `if let`
    let s = [0].as_slice();
    if let [] = s {}
    //~^ ERROR: comparison to empty slice using `if let`
    if let [] = &*s {}
    //~^ ERROR: comparison to empty slice using `if let`
    if let [] = &*s && s == [] {}
    //~^ ERROR: comparison to empty slice using `if let`
    //~| ERROR: comparison to empty slice

    // Allow comparisons to non-empty
    let s = String::new();
    let _ = s == " ";
    let _ = s != " ";

    let v = vec![0];
    let _ = v == [0];
    let _ = v != [0];
    if let [0] = &*v {}
    let s = [0].as_slice();
    if let [0] = s {}
    if let [0] = &*s && s == [0] {}
}
