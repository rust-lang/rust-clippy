#![warn(clippy::incompatible_msrv)]

#[clippy::msrv = "1.46"]
fn main() {
    if let Some((a, b)) = "foo:bar".split_once(":") {
        println!("a = {a}, b = {b}");
    }

    let x: Option<u32> = Some(42u32);
    for i in x.as_slice() {
        println!("i = {i}");
    }

    if x.is_none_or(|x| x + 2 == 17) {
        //~^ incompatible_msrv
        println!("impossible");
    }
}
