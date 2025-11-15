#![warn(clippy::map_or_identity)]

mod issue15801 {

    fn foo(opt: Option<i32>, default: i32) -> i32 {
        opt.map_or(default, |o| o)
        //~^ map_or_identity
    }

    fn bar(res: Result<i32, &str>, default: i32) -> i32 {
        res.map_or(default, |o| o)
        //~^ map_or_identity
    }
}
fn main() {
    // test code goes here
}
