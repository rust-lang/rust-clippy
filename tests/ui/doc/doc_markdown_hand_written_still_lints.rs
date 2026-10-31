// Guard against over-silencing: doc text written by the user must still be linted, including a
// raw `#[doc = "..."]` whose string literal is real source code.

#![warn(clippy::doc_markdown)]

/// Uses foo_bar here.
//~^ doc_markdown
pub fn sugared_doc_comment() {}

#[doc = "Uses foo_bar here."] //~ doc_markdown
pub fn raw_doc_attribute() {}

fn main() {}

#[doc = r#"Uses foo_bar here."#] //~ doc_markdown
pub fn raw_string_doc_attribute() {}
