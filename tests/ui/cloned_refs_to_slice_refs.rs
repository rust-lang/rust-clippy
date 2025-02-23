#![warn(clippy::cloned_refs_to_slice_refs)]

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
        take_slice(&[data_ref.clone()]); //~ ERROR: this call to clone can be replaced with `std::slice::from_ref`
    }
    {
        take_slice(&[Data.clone()]); //~ ERROR: this call to clone can be replaced with `std::slice::from_ref`
    }

    // no warning because this has different meaning
    {
        let data = Data;
        let data_ref = &data;
        take_slice_mut(&mut [data_ref.clone()]);
    }

    // no warning because this has different meaning
    {
        let data = Arc::new(Data);
        take_arc(&[Arc::clone(&data)]);
    }
}
