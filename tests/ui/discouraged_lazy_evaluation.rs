#![warn(clippy::discouraged_lazy_evaluation)]

fn foo(argument: String) {
    let _ = argument;
}

#[clippy::optional_lazy_eval = "prefer using `foo` if `argument` does not need to be computed"]
fn lazy_foo<F>(argument: F)
where
    F: FnOnce() -> String,
{
    let _ = argument();
}

fn main() {
    let s = String::from("baz");
    foo(s.clone());
    lazy_foo(|| format!("my custom string with {s}"));
    lazy_foo(move || s);
    //~^ discouraged_lazy_evaluation
}
