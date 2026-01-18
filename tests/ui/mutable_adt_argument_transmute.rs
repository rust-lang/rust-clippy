#![warn(clippy::mutable_adt_argument_transmute)]

fn main() {
    unsafe {
        let _: Option<&mut i32> = std::mem::transmute(Some(&5i32));
        //~^ mutable_adt_argument_transmute
        let _: Result<&mut i32, ()> = std::mem::transmute(Result::<&i32, ()>::Ok(&5i32));
        //~^ mutable_adt_argument_transmute
        let _: Result<Option<&mut String>, ()> =
            std::mem::transmute(Result::<Option<&String>, ()>::Ok(Some(&"foo".to_string())));
        //~^ mutable_adt_argument_transmute
        let _: Result<&mut f32, &usize> = std::mem::transmute(Result::<&f32, &usize>::Ok(&2f32));
        //~^ mutable_adt_argument_transmute
        let _: Result<(), &mut usize> = std::mem::transmute(Result::<(), &usize>::Ok(()));
        //~^ mutable_adt_argument_transmute
        let _: Option<&i32> = std::mem::transmute(Some(&5i32));
        let _: Option<(&mut i32, &mut i32, &mut u32)> = std::mem::transmute(Some((&5i32, &10i32, &15u32)));
        //~^ mutable_adt_argument_transmute
    }
}
