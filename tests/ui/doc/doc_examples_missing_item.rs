#![warn(clippy::doc_examples_missing_item)]

//! Module documentation is not checked.
//! ```
//! let _ = 1;
//! ```

/// ```
/// assert_eq!(triple(2), 6);
/// ```
pub fn double(x: u32) -> u32 {
    //~^ doc_examples_missing_item
    x * 2
}

/// ```
/// assert_eq!(triple(2), 6);
/// ```
pub fn triple(x: u32) -> u32 {
    x * 3
}

/// Documentation without an example.
pub fn no_example() {}

unsafe extern "C" {
    /// A foreign declaration has a documentation page of its own.
    /// ```
    /// let _ = 1;
    /// ```
    pub fn readv(fd: i32) -> isize;
    //~^ doc_examples_missing_item
}

/// Tagged `ignore` and `compile_fail` blocks are not examples.
/// ```text
/// text_block();
/// ```
/// ```ignore
/// text_block();
/// ```
/// ```compile_fail
/// let _: u8 = text_block;
/// ```
/// ```
/// let _ = 1;
/// ```
pub fn text_block() {}
//~^ doc_examples_missing_item

/// ```no_run
/// let _ = 1;
/// ```
pub fn no_run_block() {}
//~^ doc_examples_missing_item

/// A conditional `ignore-*` stays an example on other targets.
/// ```ignore-windows
/// let _ = 1;
/// ```
pub fn conditional_ignore() {}
//~^ doc_examples_missing_item

/// Hidden lines are compiled, so they count.
/// ```
/// # hidden_line();
/// let _ = 1;
/// ```
pub fn hidden_line() {}

/// One mention anywhere in the examples is enough.
/// ```
/// let _ = 1;
/// ```
/// ```
/// later_block();
/// ```
pub fn later_block() {}

/// ```
/// let _ = 1;
/// ```
/// ```
/// let _ = 2;
/// ```
pub fn every_block_misses() {}
//~^ doc_examples_missing_item

/// ```
/// let _ = r#type(1);
/// ```
pub fn r#type(x: u32) -> u32 {
    x
}

// Type and trait declarations are not checked.
/// ```
/// let _ = 1;
/// ```
pub enum Direction {
    Up,
}

/// ```
/// let _ = 1;
/// ```
pub union Raw {
    bits: u32,
}

/// ```
/// let _ = 1;
/// ```
pub static VERSION: &str = "1";
//~^ doc_examples_missing_item

/// ```
/// let _ = 1;
/// ```
pub const MAX: u32 = 8;
//~^ doc_examples_missing_item

/// ```
/// let _ = 1;
/// ```
pub struct Counter {
    pub ticks: u32,
}

impl Counter {
    /// ```
    /// let _ = 1;
    /// ```
    pub fn reset(&mut self) {
        //~^ doc_examples_missing_item
        self.ticks = 0;
    }

    /// ```
    /// let _ = 1;
    /// ```
    pub const LIMIT: u32 = 8;
    //~^ doc_examples_missing_item
}

/// Type aliases are not checked.
/// ```
/// let _ = 1;
/// ```
pub type Bytes = Vec<u8>;

/// ```
/// let _ = 1;
/// ```
pub trait Shape {
    /// ```
    /// let _ = 1;
    /// ```
    fn area(&self) -> u32;
    //~^ doc_examples_missing_item
}

impl Shape for Counter {
    /// Trait impls show the trait's documentation.
    /// ```
    /// let _ = 1;
    /// ```
    fn area(&self) -> u32 {
        self.ticks
    }
}

mod private {
    /// ```
    /// let _ = 1;
    /// ```
    pub fn not_exported() {}

    /// ```
    /// let _ = 1;
    /// ```
    pub fn reexported() {}
    //~^ doc_examples_missing_item
}

pub use private::reexported;

#[doc(hidden)]
pub mod hidden {
    /// ```
    /// let _ = 1;
    /// ```
    pub fn under_hidden() {}
}

#[allow(clippy::doc_examples_missing_item)]
/// ```
/// let _ = 1;
/// ```
pub fn allowed() {}
