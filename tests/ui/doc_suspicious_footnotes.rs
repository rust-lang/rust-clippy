#![warn(clippy::doc_suspicious_footnotes)]
#![allow(clippy::needless_raw_string_hashes)]
//! This is not a footnote[^1].
//~^ doc_suspicious_footnotes
//!
//! This is not a footnote[^either], but it doesn't warn.
//!
//! This is not a footnote\[^1], but it also doesn't warn.
//!
//! This is not a footnote[^1\], but it also doesn't warn.
//!
//! This is not a `footnote[^1]`, but it also doesn't warn.
//!
//! This is a footnote[^2].
//!
//! [^2]: hello world

/// This is not a footnote[^1].
//~^ doc_suspicious_footnotes
///
/// This is not a footnote[^either], but it doesn't warn.
///
/// This is not a footnote\[^1], but it also doesn't warn.
///
/// This is not a footnote[^1\], but it also doesn't warn.
///
/// This is not a `footnote[^1]`, but it also doesn't warn.
///
/// This is a footnote[^2].
///
/// [^2]: hello world
pub fn footnotes() {
    // test code goes here
}

pub struct Foo;
impl Foo {
    #[doc = r#"This is not a footnote[^1]."#]
    //~^ doc_suspicious_footnotes
    #[doc = r#""#]
    #[doc = r#"This is not a footnote[^either], but it doesn't warn."#]
    #[doc = r#""#]
    #[doc = r#"This is not a footnote\[^1], but it also doesn't warn."#]
    #[doc = r#""#]
    #[doc = r#"This is not a footnote[^1\], but it also doesn't warn."#]
    #[doc = r#""#]
    #[doc = r#"This is not a `footnote[^1]`, but it also doesn't warn."#]
    #[doc = r#""#]
    #[doc = r#"This is a footnote[^2]."#]
    #[doc = r#""#]
    #[doc = r#"[^2]: hello world"#]
    pub fn footnotes() {
        // test code goes here
    }
    #[doc = "This is not a footnote[^1].

    This is not a footnote[^either], but it doesn't warn.

    This is not a footnote\\[^1], but it also doesn't warn.

    This is not a footnote[^1\\], but it also doesn't warn.

    This is not a `footnote[^1]`, but it also doesn't warn.

    This is a footnote[^2].

    [^2]: hello world
    "]
    //~^^^^^^^^^^^^^^ doc_suspicious_footnotes
    pub fn footnotes2() {
        // test code goes here
    }
    #[cfg_attr(
        not(FALSE),
        doc = "This is not a footnote[^1].\n\nThis is not a footnote[^either], but it doesn't warn."
    //~^ doc_suspicious_footnotes
    )]
    pub fn footnotes3() {
        // test code goes here
    }
}

#[doc = r"This is not a footnote[^1]."]
//~^ doc_suspicious_footnotes
#[doc = r""]
#[doc = r"This is not a footnote[^either], but it doesn't warn."]
#[doc = r""]
#[doc = r"This is not a footnote\[^1], but it also doesn't warn."]
#[doc = r""]
#[doc = r"This is not a footnote[^1\], but it also doesn't warn."]
#[doc = r""]
#[doc = r"This is not a `footnote[^1]`, but it also doesn't warn."]
#[doc = r""]
#[doc = r"This is a footnote[^2]."]
#[doc = r""]
#[doc = r"[^2]: hello world"]
pub fn footnotes_attrs() {
    // test code goes here
}
