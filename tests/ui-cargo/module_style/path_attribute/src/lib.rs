#![warn(clippy::path_attribute)]

#[path = "parser/impl.rs"]
pub mod parser;

pub mod no_attr;

pub mod outer {
    #[path = "outer_impl.rs"]
    pub mod inner;
}
