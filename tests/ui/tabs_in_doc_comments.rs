#![warn(clippy::tabs_in_doc_comments)]
#[allow(dead_code)]

///
/// Struct to hold two strings:
//~| tabs_in_doc_comments
//~v tabs_in_doc_comments
/// 	- first		one
//~| tabs_in_doc_comments
//~v tabs_in_doc_comments
/// 	- second	one
pub struct DoubleString {
    ///
    //~v tabs_in_doc_comments
    /// 	- First String:
    //~v tabs_in_doc_comments
    /// 		- needs to be inside here
    first_string: String,
    ///
    //~v tabs_in_doc_comments
    /// 	- Second String:
    //~v tabs_in_doc_comments
    /// 		- needs to be inside here
    second_string: String,
}

/// This is main
fn main() {}
