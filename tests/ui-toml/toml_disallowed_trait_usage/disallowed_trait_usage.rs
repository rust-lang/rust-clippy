#![warn(clippy::disallowed_trait_usage)]

use std::path::{Path, PathBuf};

trait MyTrait {
    fn do_thing(&self) -> i32;
}

struct MyStruct;

impl MyTrait for MyStruct {
    fn do_thing(&self) -> i32 {
        42
    }
}

struct OtherStruct;

impl MyTrait for OtherStruct {
    fn do_thing(&self) -> i32 {
        99
    }
}

fn main() {
    // Should trigger: Debug formatting of i32
    println!("{:?}", 42_i32);
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of PathBuf
    let path = PathBuf::from("/tmp");
    println!("{path:?}");
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of &Path
    let path_ref: &Path = path.as_path();
    println!("{path_ref:?}");
    //~^ disallowed_trait_usage

    // Should NOT trigger: Display formatting of i32
    println!("{}", 42_i32);

    // Should NOT trigger: Debug formatting of String (not in config)
    let s = String::from("hello");
    println!("{s:?}");

    // Should trigger: Debug formatting of i32 via format!
    let _ = format!("{:?}", 0_i32);
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of PathBuf via write!
    use std::fmt::Write;
    let mut buf = String::new();
    write!(buf, "{path:?}").ok();
    //~^ disallowed_trait_usage

    // Should trigger: custom trait method call on custom type
    let my = MyStruct;
    my.do_thing();
    //~^ disallowed_trait_usage

    // Should trigger: custom trait method call via reference
    let my_ref = &MyStruct;
    my_ref.do_thing();
    //~^ disallowed_trait_usage

    // Should NOT trigger: same custom trait on a different type
    let other = OtherStruct;
    other.do_thing();
}
