#![warn(clippy::always_true_conditions)]
#[allow(clippy::needless_if)]
fn foo_m(name: &str) {
    if name != "Min" || name != "Max" || name != "Middle" {
        //~^ always_true_conditions
        println!("always prints");
    } else {
        println!("never prints");
    }
    if name != "Min" && name != "Max" {
        println!("condition satisfied");
    } else {
        println!("else");
    }
}

fn foo_s(name: &str) {
    if name != "Min" || name != "Max" {
        //~^ always_true_conditions
        println!("always prints");
    } else {
        println!("never prints");
    }
    if name != "Min" && name != "Max" {
        println!("condition satisfied");
    } else {
        println!("else");
    }
}

fn catch_or_failure(input: &str) {
    let b = true;
    if b || input != "foo" {
        println!("should not fire!");
    }
}

fn catch_scope_or_failures(input: &str) {
    let b = true;
    {
        if b || input != "foo" {
            println!("should not fire!");
        }
    }
}

fn catch_eq_failures() {
    let res = "test";
    if res == "foo" || res == "bar" {
        println!("should not fire!");
    }
}

fn catch_diff_var_failure(input: &str) {
    let b = "value";
    if b != "bar" || input != "foo" {
        println!("should not fire!");
    }
}
