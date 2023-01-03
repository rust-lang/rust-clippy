#![allow(unused)]
#![warn(clippy::empty_docs)]


#[doc("this is a doc")]
fn attr_doc() {

}

#[doc("")]
fn blank_attr_doc() {

}

///
fn main() {

}

///
///
fn double_empty_line_should_fire_1() {

}

/// This should not trigger
fn test_should_not_fire() {

}

/// Test function with no docs on let
fn no_docs_on_let() {
    ///
    let no_docs = 1;

    /// Here are docs
    let docs = 2;
}

///
/// This also should not trigger
fn test_should_not_fire_2(){

}

/// docs
struct StructDocs;

///
struct StructNoDocs;

struct Struct {
    /// A field
    a_field: u32,
    ///
    no_doc_field: u32,
    ///

    ///
    more_no_docs:u32,
}

union UnionFieldTest {
    /// A field
    a_union_field: u32,
    ///
    no_doc_union_field: u32,
    ///

    ///
    more_no_union_docs:u32,
}

enum ThisIsAnEnum {
    /// This variant has a docstring with text
    ThisDoes,
    ///
    ThisDoesNot
}


