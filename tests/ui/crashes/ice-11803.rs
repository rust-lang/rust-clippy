//@no-rustfix

#![warn(clippy::impl_trait_in_params)]
#![allow(clippy::could_be_assoc_type_bounds)]

pub fn g<T: IntoIterator<Item = impl Iterator<Item = impl Clone>>>() {
    extern "C" fn implementation_detail() {}
}

fn main() {}
