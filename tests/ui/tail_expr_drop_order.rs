//@compile-flags: -Z unstable-options
//@edition:2024

#![warn(clippy::tail_expr_drop_order)]

struct LoudDropper;
impl Drop for LoudDropper {
    fn drop(&mut self) {
        println!("loud drop")
    }
}
impl LoudDropper {
    fn get(&self) -> i32 {
        0
    }
}

fn should_lint() -> i32 {
    let x = LoudDropper;
    // Should lint
    x.get() + LoudDropper.get() //~ ERROR: discretion required on this expression which generates a value with a significant drop implementation
}

fn should_not_lint() -> i32 {
    let x = LoudDropper;
    // Should not lint
    x.get()
}

fn main() {}
