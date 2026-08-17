#![warn(clippy::map_or_same_constant)]
#![allow(clippy::unnecessary_map_or)]

fn main() {
    let opt: Option<i32> = Some(42);
    let res: Result<i32, ()> = Ok(42);

    // Should lint: Option::map_or with identical constants
    let _ = opt.map_or(false, |_| false);
    //~^ map_or_same_constant
    let _ = opt.map_or(true, |_| true);
    //~^ map_or_same_constant
    let _ = opt.map_or(0, |_| 0);
    //~^ map_or_same_constant
    let _ = opt.map_or("foo", |_| "foo");
    //~^ map_or_same_constant

    // Should lint: Result::map_or with identical constants
    let _ = res.map_or(false, |_| false);
    //~^ map_or_same_constant
    let _ = res.map_or(true, |_| true);
    //~^ map_or_same_constant
    let _ = res.map_or(123, |_| 123);
    //~^ map_or_same_constant

    // Should lint: Option::map_or_else with identical constants
    let _ = opt.map_or_else(|| true, |_| true);
    //~^ map_or_same_constant
    let _ = opt.map_or_else(|| 0, |_| 0);
    //~^ map_or_same_constant

    // Should lint: Result::map_or_else with identical constants
    let _ = res.map_or_else(|_| false, |_| false);
    //~^ map_or_same_constant
    let _ = res.map_or_else(|_| "same", |_| "same");
    //~^ map_or_same_constant

    // Should NOT lint: different constants
    let _ = opt.map_or(false, |_| true);
    let _ = res.map_or(0, |_| 1);
    let _ = opt.map_or_else(|| false, |_| true);

    // Should NOT lint: dynamic values
    let _ = opt.map_or(0, |x| x + 1);
    let _ = res.map_or_else(|_| 0, |x| x * 2);
}
