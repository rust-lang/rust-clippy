#![warn(clippy::assert_multiple)]
#![allow(unused)]
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn myfunc1(_a: u32, _b: String) -> bool {
    todo!()
}

struct MyStruct {}

impl MyStruct {
    fn myfunc(&self, a: u32, b: String) -> bool {
        todo!()
    }
}

fn main() {
    #[derive(PartialEq, Debug)]
    enum Vals {
        Owned,
        Borrowed,
        Other,
    }
    let o = Vals::Owned;
    let b = Vals::Borrowed;
    let other = Vals::Other;
    let is_bool = true;

    assert!(myfunc1(1, "foo".to_string()) && b == Vals::Borrowed);
    //~^ assert_multiple
    let ms = MyStruct {};
    assert!(ms.myfunc(1, "foo".to_string()) && myfunc1(2, "bar".to_string()));
    //~^ assert_multiple

    assert!(o == Vals::Owned && b == Vals::Other);
    //~^ assert_multiple

    debug_assert!(o == b && other == Vals::Other);
    //~^ assert_multiple

    assert!(o == b && (o == Vals::Owned || b == Vals::Other));
    //~^ assert_multiple
    assert!(o == b && is_bool);
    //~^ assert_multiple
    assert!(!is_bool && o == b);
    //~^ assert_multiple
    assert!(o == b && !is_bool);
    //~^ assert_multiple
    assert!(o == b && !(is_bool && o == Vals::Owned));
    //~^ assert_multiple
    let v = vec![1, 2, 3];
    assert!(v == vec![] && is_bool);
    //~^ assert_multiple

    // Next ones we cannot split.
    assert!((o == b && o == Vals::Owned) || b == Vals::Other);
    assert!(o == Vals::Owned || b == Vals::Other);
    debug_assert!(o == b);
}
