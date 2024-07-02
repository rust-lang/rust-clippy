#![feature(custom_inner_attributes)]
#![rustfmt::skip]

#![warn(clippy::doc_comment_double_space_linebreak)]
#![allow(unused)]

//! Should warn on double space linebreaks  
//! in file/module doc comment

/// Should not warn on single-line doc comments
fn single_line() {}

/// Should not warn on single-line doc comments
/// split across multiple lines
fn single_line_split() {}

// Should not warn on normal comments

// note: cargo fmt can remove double spaces from normal and block comments
// Should not warn on normal comments  
// with double spaces at the end of a line  

#[doc = "This is a doc attribute, which should not be linted"]
fn normal_comment() {
    /*
       Should not warn on block comments
    */

    /*
        Should not warn on block comments  
        with double space at the end of a line
     */
}

/// Should warn when doc comment uses double space  
/// as a line-break, even when there are multiple  
/// in a row
fn double_space_doc_comment() {}

/// Should not warn when back-slash is used \
/// as a line-break
fn back_slash_doc_comment() {}

/// 🌹 are 🟥  
/// 🌷 are 🟦  
/// 📎 is 😎  
/// and so are 🫵  
/// (hopefully no formatting weirdness linting this)
fn multi_byte_chars_tada() {}

macro_rules! macro_that_makes_function {
   () => {
      /// Shouldn't lint on this!  
      /// (hopefully)
      fn my_macro_created_function() {}
   }
}

macro_that_makes_function!();

fn main() {}
