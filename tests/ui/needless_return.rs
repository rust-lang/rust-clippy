// run-rustfix

#![feature(let_else)]
#![allow(unused)]
#![allow(
    clippy::if_same_then_else,
    clippy::single_match,
    clippy::needless_bool,
    clippy::equatable_if_let
)]
#![warn(clippy::needless_return)]

macro_rules! the_answer {
    () => {
        42
    };
}

fn test_end_of_fn() -> bool {
    if true {
        // no error!
        return true;
    }
    return true;
}

fn test_no_semicolon() -> bool {
    return true;
}

fn test_if_block() -> bool {
    if true {
        return true;
    } else {
        return false;
    }
}

fn test_match(x: bool) -> bool {
    match x {
        true => return false,
        false => {
            return true;
        },
    }
}

fn test_closure() {
    let _ = || {
        return true;
    };
    let _ = || return true;
}

fn test_macro_call() -> i32 {
    return the_answer!();
}

fn test_void_fun() {
    return;
}

fn test_void_if_fun(b: bool) {
    if b {
        return;
    } else {
        return;
    }
}

fn test_void_match(x: u32) {
    match x {
        0 => (),
        _ => return,
    }
}

fn test_nested_match(x: u32) {
    match x {
        0 => (),
        1 => {
            let _ = 42;
            return;
        },
        _ => return,
    }
}

fn read_line() -> String {
    use std::io::BufRead;
    let stdin = ::std::io::stdin();
    return stdin.lock().lines().next().unwrap().unwrap();
}

fn borrows_but_not_last(value: bool) -> String {
    if value {
        use std::io::BufRead;
        let stdin = ::std::io::stdin();
        let _a = stdin.lock().lines().next().unwrap().unwrap();
        return String::from("test");
    } else {
        return String::new();
    }
}

macro_rules! needed_return {
    ($e:expr) => {
        if $e > 3 {
            return;
        }
    };
}

fn test_return_in_macro() {
    // This will return and the macro below won't be executed. Removing the `return` from the macro
    // will change semantics.
    needed_return!(10);
    needed_return!(0);
}

mod issue6501 {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    fn foo(bar: Result<(), ()>) {
        bar.unwrap_or_else(|_| return)
    }

    fn test_closure() {
        let _ = || {
            return;
        };
        let _ = || return;
    }

    struct Foo;
    #[allow(clippy::unnecessary_lazy_evaluations)]
    fn bar(res: Result<Foo, u8>) -> Foo {
        res.unwrap_or_else(|_| return Foo)
    }
}

async fn async_test_end_of_fn() -> bool {
    if true {
        // no error!
        return true;
    }
    return true;
}

async fn async_test_no_semicolon() -> bool {
    return true;
}

async fn async_test_if_block() -> bool {
    if true {
        return true;
    } else {
        return false;
    }
}

async fn async_test_match(x: bool) -> bool {
    match x {
        true => return false,
        false => {
            return true;
        },
    }
}

async fn async_test_closure() {
    let _ = || {
        return true;
    };
    let _ = || return true;
}

async fn async_test_macro_call() -> i32 {
    return the_answer!();
}

async fn async_test_void_fun() {
    return;
}

async fn async_test_void_if_fun(b: bool) {
    if b {
        return;
    } else {
        return;
    }
}

async fn async_test_void_match(x: u32) {
    match x {
        0 => (),
        _ => return,
    }
}

async fn async_read_line() -> String {
    use std::io::BufRead;
    let stdin = ::std::io::stdin();
    return stdin.lock().lines().next().unwrap().unwrap();
}

async fn async_borrows_but_not_last(value: bool) -> String {
    if value {
        use std::io::BufRead;
        let stdin = ::std::io::stdin();
        let _a = stdin.lock().lines().next().unwrap().unwrap();
        return String::from("test");
    } else {
        return String::new();
    }
}

async fn async_test_return_in_macro() {
    needed_return!(10);
    needed_return!(0);
}

fn let_else() {
    let Some(1) = Some(1) else { return };
}

fn main() {}
