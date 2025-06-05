#![warn(clippy::doc_link_with_quotes)]
#![allow(clippy::disallowed_names)]

fn main() {
    foo()
}

/// Calls ['bar'] uselessly
//~^ doc_link_with_quotes
pub fn foo() {
    bar()
}

/// Calls ["bar"] uselessly
//~^ doc_link_with_quotes
pub fn foo2() {
    bar()
}

/// # Examples
/// This demonstrates issue \#8961
/// ```
/// let _ = vec!['w', 'a', 't'];
/// ```
pub fn bar() {}
