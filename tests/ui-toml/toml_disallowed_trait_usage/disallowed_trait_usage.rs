#![warn(clippy::disallowed_trait_usage)]
#![allow(clippy::io_other_error)]

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

impl std::fmt::Debug for MyStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MyStruct")
    }
}

struct OtherStruct;

impl MyTrait for OtherStruct {
    fn do_thing(&self) -> i32 {
        99
    }
}

fn main() {
    // === Concrete `type` matching ===

    // Should trigger: Debug formatting of i32
    println!("{:?}", 42_i32);
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of PathBuf
    let path = PathBuf::from("/tmp");
    println!("{path:?}");
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of &Path (references are peeled)
    let path_ref: &Path = path.as_path();
    println!("{path_ref:?}");
    //~^ disallowed_trait_usage

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

    // Should NOT trigger: Display formatting of i32 (only Debug is disallowed)
    println!("{}", 42_i32);

    // Should NOT trigger: Debug formatting of String (not in config)
    let s = String::from("hello");
    println!("{s:?}");

    // Should NOT trigger: same custom trait on a different type (OtherStruct not in config)
    let other = OtherStruct;
    other.do_thing();

    // === `implements` matching ===

    // Should trigger: Debug formatting of std::io::Error (implements std::error::Error)
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    println!("{err:?}");
    //~^ disallowed_trait_usage

    // Should trigger: Debug formatting of Error via format!
    let _ = format!("{:?}", std::io::Error::new(std::io::ErrorKind::Other, "oops"));
    //~^ disallowed_trait_usage

    // Should NOT trigger: Display formatting of Error (only Debug is disallowed)
    println!("{err}");

    // Should NOT trigger: Debug formatting of String (doesn't implement Error)
    println!("{s:?}");

    // Should trigger: Debug formatting of MyStruct (implements MyTrait, which is in `implements`
    // config)
    println!("{my:?}");
    //~^ disallowed_trait_usage

    // Should NOT trigger: OtherStruct implements MyTrait but doesn't impl Debug,
    // so Debug formatting can't even be used on it (won't compile without this guard).
    // Instead, test that Display of MyStruct doesn't trigger (only Debug is disallowed via
    // `implements`). (MyStruct has no Display impl, so we test via the method call path instead.)

    // Should trigger: method call on a type matching `implements` —
    // OtherStruct implements MyTrait, and MyTrait::do_thing is disallowed on MyStruct (via concrete
    // `type`), but OtherStruct is NOT matched by the concrete `type` entry. However, it IS matched
    // by the `implements = MyTrait` + `trait = Debug` entry — but that only covers Debug, not
    // MyTrait methods. So this should NOT trigger.
    other.do_thing();
}
