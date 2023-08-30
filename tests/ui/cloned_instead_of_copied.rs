#![warn(clippy::cloned_instead_of_copied)]
#![allow(unused)]
#![allow(clippy::useless_vec)]

fn main() {
    // yay
    let _ = [1].iter().cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead
    //~| NOTE: `-D clippy::cloned-instead-of-copied` implied by `-D warnings`
    let _ = vec!["hi"].iter().cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead
    let _ = Some(&1).cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead
    let _ = Box::new([1].iter()).cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead
    let _ = Box::new(Some(&1)).cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead

    // nay
    let _ = [String::new()].iter().cloned();
    let _ = Some(&String::new()).cloned();
}

#[clippy::msrv = "1.34"]
fn msrv_1_34() {
    let _ = [1].iter().cloned();
    let _ = Some(&1).cloned();
}

#[clippy::msrv = "1.35"]
fn msrv_1_35() {
    let _ = [1].iter().cloned();
    let _ = Some(&1).cloned(); // Option::copied needs 1.35
    //~^ ERROR: used `cloned` where `copied` could be used instead
}

#[clippy::msrv = "1.36"]
fn msrv_1_36() {
    let _ = [1].iter().cloned(); // Iterator::copied needs 1.36
    //~^ ERROR: used `cloned` where `copied` could be used instead
    let _ = Some(&1).cloned();
    //~^ ERROR: used `cloned` where `copied` could be used instead
}
