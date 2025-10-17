#![warn(clippy::trivial_var_primitive_disjunction)]
#![allow(clippy::nonminimal_bool, clippy::eq_op)]


fn foo_m(name: &str) {
    if name != "Min" && name != "Max" {
        todo!()
    } else {
        todo!()
    }
}

fn foo_s(name: &str) {
    if name != "Min" || name != "Max" {
        //~^ trivial_var_primitive_disjunction
    }
    if name != "Min" && name != "Max" {
        todo!()
    }
}

fn ne_ne(num: u32) {
    if num != 1 || num != 1 {
        todo!()
    }
}

fn simple_op(name: &str) {
    if name != "Min" || name != "Max" && name != "Mid" {
        todo!()
    }
    if name == "Min" || name == "Min" {
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
        //~^ trivial_var_primitive_disjunction
        todo!()
    }

    if b != 3 || 5 != b {
        //~^ trivial_var_primitive_disjunction
        todo!()
    }
}

fn non_if_funcitonality_tests(input: &str) {
    let x: bool = ("a" != input || "b" != input);
    //~^ trivial_var_primitive_disjunction
    let z: bool = (input != "2" || input != "4");
    //~^ trivial_var_primitive_disjunction
}

fn redundant_comparision(z: u8) {
    let b: bool = (1 != z || z != 1);
}
