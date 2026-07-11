//@no-rustfix: the suggestion is intentionally not machine-applicable here
#![warn(clippy::let_else_ok_or)]
#![allow(dead_code, unused, clippy::question_mark)]

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

// The error value is not a constant: `?` converts it through `From::from`, so the target of
// `.into()` can no longer be inferred. Only a non-machine-applicable `ok_or_else` is suggested.
fn inference(opt: Option<u8>) -> Result<u8, String> {
    //~v let_else_ok_or
    let Some(n) = opt else {
        return Err("missing".into());
    };
    Ok(n)
}

// The error value borrows the slice that `first_mut()` keeps mutably borrowed on the `Some`
// path: the rewrite would not compile (E0502).
fn borrow(slice: &mut [u8]) -> Result<u8, usize> {
    //~v let_else_ok_or
    let Some(n) = slice.first_mut() else {
        return Err(slice.len());
    };
    *n += 1;
    Ok(*n)
}

// The error value has side effects, which must only happen on `None`: suggest `ok_or_else`,
// never `ok_or`.
fn eager(opt: Option<u8>) -> Result<u8, u8> {
    let mut counter = 0;
    //~v let_else_ok_or
    let Some(n) = opt else {
        return Err({
            counter += 1;
            counter
        });
    };
    Ok(n + counter)
}

// An unsuffixed integer literal gets its type from the expected error type.
fn unsuffixed_literal(opt: Option<u8>) -> Result<u8, u64> {
    //~v let_else_ok_or
    let Some(n) = opt else {
        return Err(1);
    };
    Ok(n)
}

// A `Copy` local can be read eagerly, but may still conflict with a borrow held by the `Option`.
fn copy_local(opt: Option<u8>, code: u32) -> Result<u8, u32> {
    //~v let_else_ok_or
    let Some(n) = opt else {
        return Err(code);
    };
    Ok(n)
}

// A constant with a destructor would be built and dropped on the `Some` path too, so the value
// must stay lazy.
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop");
    }
}
const GUARD: Guard = Guard;

fn const_with_drop(opt: Option<u8>) -> Result<u8, Guard> {
    //~v let_else_ok_or
    let Some(n) = opt else {
        return Err(GUARD);
    };
    Ok(n)
}

fn main() {}
