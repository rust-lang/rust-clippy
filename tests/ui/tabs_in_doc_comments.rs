#![warn(clippy::tabs_in_doc_comments)]
#[allow(dead_code)]

///
/// Struct to hold two strings:
/// 	- first		one
//~^ ERROR: using tabs in doc comments is not recommended
//~| ERROR: using tabs in doc comments is not recommended
/// 	- second	one
//~^ ERROR: using tabs in doc comments is not recommended
//~| ERROR: using tabs in doc comments is not recommended
pub struct DoubleString {
    ///
    /// 	- First String:
    //~^ ERROR: using tabs in doc comments is not recommended
    //~| NOTE: `-D clippy::tabs-in-doc-comments` implied by `-D warnings`
    /// 		- needs to be inside here
    //~^ ERROR: using tabs in doc comments is not recommended
    first_string: String,
    ///
    /// 	- Second String:
    //~^ ERROR: using tabs in doc comments is not recommended
    /// 		- needs to be inside here
    //~^ ERROR: using tabs in doc comments is not recommended
    second_string: String,
}

/// This is main
fn main() {}
