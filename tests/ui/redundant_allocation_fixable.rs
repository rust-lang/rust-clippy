#![warn(clippy::all)]
#![allow(clippy::boxed_local, clippy::needless_pass_by_value)]
#![allow(clippy::disallowed_names, unused_variables, dead_code)]
#![allow(unused_imports)]

pub struct MyStruct;

pub struct SubT<T> {
    foo: T,
}

pub enum MyEnum {
    One,
    Two,
}

mod outer_box {
    use crate::{MyEnum, MyStruct, SubT};
    use std::boxed::Box;
    use std::rc::Rc;
    use std::sync::Arc;

    pub fn box_test1<T>(foo: Box<&T>) {}
    //~^ ERROR: usage of `Box<&T>`
    //~| NOTE: `&T` is already a pointer, `Box<&T>` allocates a pointer on the heap

    pub fn box_test2(foo: Box<&MyStruct>) {}
    //~^ ERROR: usage of `Box<&MyStruct>`
    //~| NOTE: `&MyStruct` is already a pointer, `Box<&MyStruct>` allocates a pointer on t

    pub fn box_test3(foo: Box<&MyEnum>) {}
    //~^ ERROR: usage of `Box<&MyEnum>`
    //~| NOTE: `&MyEnum` is already a pointer, `Box<&MyEnum>` allocates a pointer on the h

    pub fn box_test4_neg(foo: Box<SubT<&usize>>) {}

    pub fn box_test5<T>(foo: Box<Box<T>>) {}
    //~^ ERROR: usage of `Box<Box<T>>`
    //~| NOTE: `Box<T>` is already on the heap, `Box<Box<T>>` makes an extra allocation
}

mod outer_rc {
    use crate::{MyEnum, MyStruct, SubT};
    use std::boxed::Box;
    use std::rc::Rc;
    use std::sync::Arc;

    pub fn rc_test1<T>(foo: Rc<&T>) {}
    //~^ ERROR: usage of `Rc<&T>`
    //~| NOTE: `&T` is already a pointer, `Rc<&T>` allocates a pointer on the heap

    pub fn rc_test2(foo: Rc<&MyStruct>) {}
    //~^ ERROR: usage of `Rc<&MyStruct>`
    //~| NOTE: `&MyStruct` is already a pointer, `Rc<&MyStruct>` allocates a pointer on th

    pub fn rc_test3(foo: Rc<&MyEnum>) {}
    //~^ ERROR: usage of `Rc<&MyEnum>`
    //~| NOTE: `&MyEnum` is already a pointer, `Rc<&MyEnum>` allocates a pointer on the he

    pub fn rc_test4_neg(foo: Rc<SubT<&usize>>) {}

    pub fn rc_test6(a: Rc<Rc<bool>>) {}
    //~^ ERROR: usage of `Rc<Rc<bool>>`
    //~| NOTE: `Rc<bool>` is already on the heap, `Rc<Rc<bool>>` makes an extra allocation
}

mod outer_arc {
    use crate::{MyEnum, MyStruct, SubT};
    use std::boxed::Box;
    use std::rc::Rc;
    use std::sync::Arc;

    pub fn arc_test1<T>(foo: Arc<&T>) {}
    //~^ ERROR: usage of `Arc<&T>`
    //~| NOTE: `&T` is already a pointer, `Arc<&T>` allocates a pointer on the heap

    pub fn arc_test2(foo: Arc<&MyStruct>) {}
    //~^ ERROR: usage of `Arc<&MyStruct>`
    //~| NOTE: `&MyStruct` is already a pointer, `Arc<&MyStruct>` allocates a pointer on t

    pub fn arc_test3(foo: Arc<&MyEnum>) {}
    //~^ ERROR: usage of `Arc<&MyEnum>`
    //~| NOTE: `&MyEnum` is already a pointer, `Arc<&MyEnum>` allocates a pointer on the h

    pub fn arc_test4_neg(foo: Arc<SubT<&usize>>) {}

    pub fn arc_test7(a: Arc<Arc<bool>>) {}
    //~^ ERROR: usage of `Arc<Arc<bool>>`
    //~| NOTE: `Arc<bool>` is already on the heap, `Arc<Arc<bool>>` makes an extra allocat
}

fn main() {}
