#![warn(clippy::always_true_conditions)]

fn foo_m(name: &str) {
    if name != "Min" && name != "Max" {
        todo!()
    } else {
        todo!()
    }
}

fn foo_s(name: &str) {
    if name != "Min" || name != "Max" {
        //~^ always_true_conditions
        todo!()
    }
    if name != "Min" && name != "Max" {
        todo!()
    }
}

fn catch_or_failure(input: &str) {
    let b = true;
    if b || input != "foo" {
        todo!()
    }
}

fn catch_scope_or_failures(input: &str) {
    let b = true;
    { if b || input != "foo" {} }
}

fn catch_eq_failures() {
    let res = "test";
    if res == "foo" || res == "bar" {
        todo!()
    }
}

fn catch_diff_var_failure(input: &str) {
    let b = "value";
    if b != "bar" || input != "foo" {
        todo!()
    }
}

fn catch_yoda_notation(input: &str) {
    let b = 2;
    if 3 != b || 5 != b {
        //~^ always_true_conditions
        todo!()
    }

    if b != 3 || 5 != b {
        //~^ always_true_conditions
        todo!()
    }
}

fn non_if_funcitonality_tests(input: &str) {
    let x: bool = ("a" != input || "b" != input);
    //~^ always_true_conditions
    let y: bool = ("a" == input || "b" != input);
    let z: bool = (input != "2" || input != "4");
    //~^ always_true_conditions
}
