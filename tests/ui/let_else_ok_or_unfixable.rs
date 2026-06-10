//@no-rustfix: the suggestion is intentionally not machine-applicable here
#![warn(clippy::let_else_ok_or)]
#![allow(clippy::question_mark, dead_code, unused)]

// The `else` value relies on a coercion to the function's error type that `?` cannot reproduce: the
// `return Err(b"abc")` coerces `&[u8; 3]` to `&[u8]`, but `opt.ok_or(b"abc")?` would need
// `From<&[u8; 3]>` for `&[u8]`, which does not exist, so the rewrite would not compile. The lint
// still fires (the rewrite is usually a one-character annotation away from correct), but the
// suggestion is downgraded to non-machine-applicable so `--fix` leaves the code untouched.
fn coerced_array_to_slice(opt: Option<i32>) -> Result<i32, &'static [u8]> {
    //~v let_else_ok_or
    let Some(value) = opt else {
        return Err(b"abc");
    };
    Ok(value)
}

// The error value *moves* a non-`Copy` local that the `Some` path still uses. `ok_or`/`ok_or_else`
// move the error value unconditionally, so `opt.ok_or(msg)?` would leave the later `msg` referencing
// a moved value and fail to compile. The lint still fires, but the suggestion is downgraded to
// non-machine-applicable so `--fix` leaves it untouched.
fn moves_value_used_later(opt: Option<i32>, msg: String) -> Result<i32, String> {
    //~v let_else_ok_or
    let Some(value) = opt else {
        return Err(msg);
    };
    println!("{msg}");
    Ok(value)
}

fn main() {}
