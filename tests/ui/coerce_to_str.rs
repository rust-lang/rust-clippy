#![warn(clippy::coerce_to_str)]
#![allow(clippy::needless_borrows_for_generic_args)]

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

trait TestTrait<T>
where
    T: AsRef<str>,
{
    fn test_fn(&self, _other_arg: i32, _string: T) {}
}

trait TestTrait2 {
    fn test_fn_2<T: AsRef<str>>(&self, _string: T) {}
}

struct TestStruct;

impl<T: AsRef<str>> TestTrait<T> for TestStruct {}
impl TestTrait2 for TestStruct {}

fn test_fn<T: AsRef<str>>(_other_arg: i32, _string: T) {}

fn test_fn_2(_str: &str) {}

fn main() {
    // Methods
    let mut s = String::from("hello");
    s.push_str(&String::from(" world"));
    //~^ coerce_to_str

    let test_struct = TestStruct;
    test_struct.test_fn(42, &String::from("hello"));
    //~^ coerce_to_str

    let test_struct = TestStruct;
    test_struct.test_fn_2(&String::from("hello"));
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

    // Functions
    test_fn(42, &String::from("hello"));
    //~^ coerce_to_str

    test_fn(42, &Into::<String>::into("hello"));
    //~^ coerce_to_str

    test_fn(42, &Cow::from("hello"));
    //~^ coerce_to_str

    test_fn(42, &Cow::Borrowed("hello"));
    //~^ coerce_to_str

    test_fn(42, &Into::<Cow<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(42, &Box::from("hello"));
    //~^ coerce_to_str

    test_fn(42, &Into::<Box<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(42, &Rc::from("hello"));
    //~^ coerce_to_str

    test_fn(42, &Into::<Rc<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn(42, &Arc::from("hello"));
    //~^ coerce_to_str

    test_fn(42, &Into::<Arc<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&String::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&Into::<String>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&Cow::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&Cow::Borrowed("hello"));
    //~^ coerce_to_str

    test_fn_2(&Into::<Cow<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&Box::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&Box::new("hello"));
    //~^ coerce_to_str

    test_fn_2(&Into::<Box<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&Rc::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&Rc::new("hello"));
    //~^ coerce_to_str

    test_fn_2(&Into::<Rc<_>>::into("hello"));
    //~^ coerce_to_str

    test_fn_2(&Arc::from("hello"));
    //~^ coerce_to_str

    test_fn_2(&Arc::new("hello"));
    //~^ coerce_to_str

    test_fn_2(&Into::<Arc<_>>::into("hello"));
    //~^ coerce_to_str
}
