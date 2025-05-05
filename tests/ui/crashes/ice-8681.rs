//@ check-pass
//@aux-build: ice-8681-aux.rs

#![warn(clippy::undocumented_unsafe_blocks)]

#[path = "auxiliary/ice-8681-aux.rs"]
mod ice_8681_aux;

fn main() {
    let _ = ice_8681_aux::some_meaningful_name(&0u32);
}
