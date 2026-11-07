#![warn(clippy::coerce_to_str)]
#![allow(clippy::needless_borrows_for_generic_args, clippy::ptr_arg)]

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

trait TestTrait {
    fn test_fn(&self, _other_arg: i32, _str_ptr: *const str) {}
}

struct TestStruct;

impl TestTrait for TestStruct {}

fn test_fn(_str: &str) {}

fn test_fn_2(s: *const str) {}

fn test_fn_3(s: &String) {}

fn main() {
    // Methods
    let mut s = String::from("hello");
    s.push_str(&String::from(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Into::<String>::into(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Cow::from(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Cow::Borrowed(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Into::<Cow<_>>::into(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Box::from(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Box::new(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Into::<Box<_>>::into(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Rc::from(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Rc::new(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Into::<Rc<_>>::into(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Arc::from(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Arc::new(" world"));
    //~^ coerce_to_str

    let mut s = String::from("hello");
    s.push_str(&Into::<Arc<_>>::into(" world"));
    //~^ coerce_to_str

    let test_struct = TestStruct;

    test_struct.test_fn(42, &*String::from("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, &*Into::<String>::into("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Box::<&str>::from("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Box::<&str>::new("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Into::<Box<&str>>::into("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Rc::<&str>::from("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Rc::<&str>::new("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Into::<Rc<&str>>::into("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Arc::<&str>::from("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Arc::<&str>::new("hello"));
    //~^ coerce_to_str

    test_struct.test_fn(42, *Into::<Arc<&str>>::into("hello"));
    //~^ coerce_to_str

    // Functions
    test_fn(&String::from("hello"));
    //~^ coerce_to_str

    test_fn(&Into::<String>::into("hello"));
    //~^ coerce_to_str

    test_fn(&Cow::from("hello"));
    //~^ coerce_to_str

    test_fn(&Cow::Borrowed("hello"));
    //~^ coerce_to_str

    test_fn(&Into::<Cow<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(&Box::from("hello"));
    //~^ coerce_to_str

    test_fn(&Box::new("hello"));
    //~^ coerce_to_str

    test_fn(&Into::<Box<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(&Rc::from("hello"));
    //~^ coerce_to_str

    test_fn(&Rc::new("hello"));
    //~^ coerce_to_str

    test_fn(&Into::<Rc<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(&Arc::from("hello"));
    //~^ coerce_to_str

    test_fn(&Arc::new("hello"));
    //~^ coerce_to_str

    test_fn(&Into::<Arc<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&*String::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&*Into::<String>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(*Box::<&str>::from("hello"));
    //~^ coerce_to_str

    test_fn_2(*Box::<&str>::new("hello"));
    //~^ coerce_to_str

    test_fn_2(*Into::<Box<&str>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(*Rc::<&str>::from("hello"));
    //~^ coerce_to_str

    test_fn_2(*Rc::<&str>::new("hello"));
    //~^ coerce_to_str

    test_fn_2(*Into::<Rc<&str>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(*Arc::<&str>::from("hello"));
    //~^ coerce_to_str

    test_fn_2(*Arc::<&str>::new("hello"));
    //~^ coerce_to_str

    test_fn_2(*Into::<Arc<&str>>::into("hello"));
    //~^ coerce_to_str

    // Should NOT lint
    test_fn_3(&String::from("hello"));
}
