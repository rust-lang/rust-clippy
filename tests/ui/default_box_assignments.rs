#![warn(clippy::default_box_assignments)]

macro_rules! create_default {
    () => {
        Default::default()
    };
}

fn main() {
    let mut b = Box::new(1u32);
    b = Default::default();
    //~^ default_box_assignments
    b = Box::default();
    //~^ default_box_assignments

    // No lint for assigning to the storage
    *b = Default::default();
    *b = u32::default();

    // No lint for call originating in macro
    b = create_default!();

    // No lint for assigning to Box<T> where T: !Default
    let mut b = Box::<str>::from("hi".to_string());
    b = Default::default();
}
