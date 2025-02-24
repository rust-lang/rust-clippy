#![warn(clippy::cloned_ref_to_slice_refs)]

use std::sync::Arc;

fn take_slice(_data: &[Data]) {}
fn take_slice_mut(_data: &mut [Data]) {}
fn take_arc(_data: &[Arc<Data>]) {}

#[derive(Clone)]
struct Data;

fn main() {
    {
        let data = Data;
        let data_ref = &data;
        take_slice(&[data_ref.clone()]); //~ ERROR: this call to `clone` can be replaced with `std::slice::from_ref`
    }
    {
        take_slice(&[Data.clone()]); //~ ERROR: this call to `clone` can be replaced with `std::slice::from_ref`
    }

    // no warning mutable borrows may have the intention to clone
    {
        let data = Data;
        let data_ref = &data;
        take_slice_mut(&mut [data_ref.clone()]);
    }

    // no warning `Ty::clone` is used to denote a clone with side effects
    {
        let data = Arc::new(Data);
        take_arc(&[Arc::clone(&data)]);
    }

    // no warning slices with multiple members cannot be made from a singular reference
    {
        let data_1 = Data;
        let data_2 = Data;
        take_slice(&[data_1.clone(), data_2.clone()]);
    }
}
