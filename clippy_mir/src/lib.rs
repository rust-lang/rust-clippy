#![feature(anonymous_lifetime_in_impl_trait, cmp_minmax, if_let_guard, rustc_private)]

extern crate rustc_abi;
extern crate rustc_arena;
extern crate rustc_data_structures;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_span;

pub mod analysis;
pub mod childless_projection;
pub mod projection;
pub mod value_tracking;
