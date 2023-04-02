// run-rustfix
#![allow(unused)]
#![warn(clippy::safe_unchecked)]

fn should_lint_unchecked() {}
fn should_not_lint() {} // Name doesn't end with "_unchecked"
unsafe fn should_not_lint_unchecked() {} // It's an unsafe function

trait SomeTrait {
    fn method_should_not_lint_unchecked();
}

impl SomeTrait for usize {
    fn method_should_not_lint_unchecked() {}
}

fn main() {}
