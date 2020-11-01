// edition:2018

#![warn(clippy::rebind_fn_arg_as_mut)]

fn f(x: bool) {
    let mut x = x;
}

trait T {
    fn tm1(x: bool) {
        let mut x = x;
    }
    fn tm2(x: bool);
}

struct S;

impl S {
    fn m(x: bool) {
        let mut x = x;
    }
}

impl T for S {
    fn tm2(x: bool) {
        let mut x = x;
    }
}

fn f_no_lint(mut x: bool) {
    let mut x = x;
}

async fn expansion<T>(_: T) {}

fn main() {}
