#![warn(clippy::eager_fun_call)]

#[clippy::avoid_eager_arguments = "the value of `argument` may not always be used, prefer using `lazy_foo` instead"]
fn foo(argument: String) {
    let _ = argument;
}

fn lazy_foo<F>(argument: F)
where
    F: Fn() -> String,
{
    let _ = argument();
}

fn main() {
    let s = String::from("baz");
    foo(String::from("bar"));
    //~^ eager_fun_call
    foo(s);
    lazy_foo(|| String::from("bar"));
}
