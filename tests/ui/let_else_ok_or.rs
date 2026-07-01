#![warn(clippy::let_else_ok_or)]
#![allow(
    clippy::let_and_return,
    clippy::diverging_sub_expression,
    clippy::question_mark,
    dead_code,
    unused
)]

use std::ffi::OsString;

// Trivial error value -> `ok_or`.
fn trivial(opt: Option<i32>) -> Result<i32, &'static str> {
    //~v let_else_ok_or
    let Some(value) = opt else {
        return Err("missing");
    };
    Ok(value)
}

// Allocating/expensive error value -> `ok_or_else` to preserve laziness.
fn allocating(opt: Option<i32>) -> Result<i32, String> {
    //~v let_else_ok_or
    let Some(value) = opt else {
        return Err("missing value".to_string());
    };
    Ok(value)
}

// The real-world motivating case (uutils/diffutils#243).
fn parse_params(mut opts: impl Iterator<Item = OsString>) -> Result<String, String> {
    let executable = opts.next().ok_or("Usage: <exe> <from> <to>".to_string())?;
    Ok(executable.to_string_lossy().to_string())
}

fn from_call(opts: &mut impl Iterator<Item = OsString>) -> Result<OsString, String> {
    //~v let_else_ok_or
    let Some(executable) = opts.next() else {
        return Err("Usage: <exe> <from> <to>".to_string());
    };
    Ok(executable)
}

// `mut` binding should be preserved.
fn mut_binding(opt: Option<Vec<i32>>) -> Result<Vec<i32>, &'static str> {
    //~v let_else_ok_or
    let Some(mut value) = opt else {
        return Err("empty");
    };
    value.push(1);
    Ok(value)
}

// Method-call receiver needs no extra parentheses.
fn method_receiver(s: &str) -> Result<char, &'static str> {
    //~v let_else_ok_or
    let Some(first) = s.chars().next() else {
        return Err("empty string");
    };
    Ok(first)
}

// The error value only *borrows* a local that is used later (it does not move it), so the rewrite
// stays machine-applicable even though `msg` lives on past the statement.
fn borrows_value_used_later(opt: Option<i32>, msg: String) -> Result<i32, String> {
    //~v let_else_ok_or
    let Some(value) = opt else {
        return Err(format!("missing: {msg}"));
    };
    println!("{msg}");
    Ok(value)
}

//
// Cases that must NOT lint.
//

// `else` does more than return an `Err`.
fn else_has_side_effects(opt: Option<i32>) -> Result<i32, &'static str> {
    let Some(value) = opt else {
        eprintln!("nothing here");
        return Err("missing");
    };
    Ok(value)
}

// `else` returns `None`, not `Err` (handled by `question_mark`).
fn returns_none(opt: Option<i32>) -> Option<i32> {
    let Some(value) = opt else {
        return None;
    };
    Some(value)
}

// `else` returns a non-`Err` value.
fn returns_default(opt: Option<i32>) -> i32 {
    let Some(value) = opt else {
        return 0;
    };
    value
}

// Comments in the `else` branch would be lost.
fn has_comment(opt: Option<i32>) -> Result<i32, &'static str> {
    let Some(value) = opt else {
        // important explanation
        return Err("missing");
    };
    Ok(value)
}

// Binding by reference cannot be rewritten as `ok_or`.
fn by_ref(opt: &Option<i32>) -> Result<i32, &'static str> {
    let Some(value) = opt else {
        return Err("missing");
    };
    Ok(*value)
}

// The `else` branch is a macro expanding to `return Err(..)`; the error value lives inside the
// macro, so the suggestion must not reach into it (e.g. `anyhow::bail!`).
macro_rules! bail {
    ($msg:literal) => {
        return Err($msg)
    };
}

fn else_is_bail_macro(opt: Option<i32>) -> Result<i32, &'static str> {
    let Some(value) = opt else {
        bail!("missing");
    };
    Ok(value)
}

// Inside a macro the suggestion would not be applicable.
macro_rules! bind_or_err {
    ($opt:expr) => {{
        let Some(value) = $opt else {
            return Err("missing");
        };
        value
    }};
}

fn in_macro(opt: Option<i32>) -> Result<i32, &'static str> {
    let value = bind_or_err!(opt);
    Ok(value)
}

// In a `const` context the suggestion (`opt.ok_or(..)?`) would not compile, as `Option::ok_or`
// and `?` are not yet const-stable, so the lint must stay silent here.
const fn in_const(opt: Option<i32>) -> Result<i32, ()> {
    let Some(value) = opt else {
        return Err(());
    };
    Ok(value)
}

fn main() {}
