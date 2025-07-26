#![warn(clippy::comment_within_doc)]

//! Hello
// oups
//! tadam
//~^^^ comment_within_doc

/// Hello
// oups
/// hehe
//~^^^ comment_within_doc
struct Bar;

mod b {
    //! targe
    // oups
    //! hello
    //
    /// nope
    // oups
    /// yep
    //~^^^ comment_within_doc
    //~^^^^^^^^ comment_within_doc
    struct Bar;
}

fn main() {}
