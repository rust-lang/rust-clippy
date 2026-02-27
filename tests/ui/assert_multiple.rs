#![warn(clippy::assert_multiple)]
#![allow(unused)]
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn myfunc(_a: u32, _b: String) -> bool {
    let time1 = SystemTime::now();
    let one_sec = Duration::from_secs(1);
    sleep(one_sec);

    time1.elapsed().unwrap() >= one_sec
}

fn main() {
    #[derive(PartialEq)]
    enum Vals {
        Owned,
        Borrowed,
        Other,
    }
    let o = Vals::Owned;
    let b = Vals::Borrowed;
    let other = Vals::Other;
    let time = SystemTime::now();
    let one_sec = Duration::from_secs(1);
    sleep(one_sec);
    let elp = time.elapsed().unwrap();

    assert!(myfunc(1, "foo".to_string()) && b == Vals::Borrowed);
    //~^ assert_multiple
}
