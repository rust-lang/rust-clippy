#![warn(clippy::mutable_adt_argument_transmute)]

fn main() {
    unsafe {
        let _: Option<&mut i32> = std::mem::transmute(Some(&5i32));
        //~^ mutable_adt_argument_transmute
    }
}
