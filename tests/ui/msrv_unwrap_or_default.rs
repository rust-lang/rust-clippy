#![warn(clippy::unwrap_or_default)]

#[clippy::msrv = "1.15"]
fn f(foo: Result<Vec<u32>, &'static str>) -> Vec<u32> {
    foo.unwrap_or(vec![])
}

fn main() {}
