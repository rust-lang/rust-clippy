#![feature(try_blocks)]
#![warn(clippy::manual_map_err)]
#![allow(
    unused,
    clippy::manual_let_else,
    clippy::needless_late_init,
    clippy::needless_question_mark,
    clippy::question_mark,
    clippy::redundant_closure,
    clippy::unnecessary_wraps
)]

#[derive(Debug)]
struct Error;

#[derive(Debug)]
enum IfFrom {
    All,
    Group(u32),
}

fn parse(s: &str) -> Result<u32, std::num::ParseIntError> {
    s.parse()
}

// The case from uutils/coreutils#13620: the `Ok` arm wraps the value, the `Err` arm returns an
// unrelated error.
fn wrapping_ok_arm(s: &str) -> Result<IfFrom, Error> {
    let filter = match parse(s) {
        //~^ manual_map_err
        Ok(g) => IfFrom::Group(g),
        Err(_) => {
            return Err(Error);
        },
    };
    Ok(filter)
}

// Identity `Ok` arm: only `map_err` is needed.
fn identity_ok_arm(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        //~^ manual_map_err
        Ok(v) => v,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The bound error is used to build a different error.
fn uses_err_binding(s: &str) -> Result<u32, String> {
    let v = match parse(s) {
        //~^ manual_map_err
        Ok(v) => v * 2,
        Err(e) => return Err(format!("bad: {e}")),
    };
    Ok(v)
}

// Arms in reverse order.
fn reversed_arms(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        //~^ manual_map_err
        Err(_) => return Err(Error),
        Ok(v) => v + 1,
    };
    Ok(v)
}

// Braced `Ok` arm body.
fn braced_ok_body(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        //~^ manual_map_err
        Ok(v) => {
            let doubled = v * 2;
            doubled + 1
        },
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The `if let` form.
fn if_let_form(s: &str) -> Result<IfFrom, Error> {
    let filter = if let Ok(g) = parse(s) {
        //~^ manual_map_err
        IfFrom::Group(g)
    } else {
        return Err(Error);
    };
    Ok(filter)
}

// Not in a `let`: the match is the tail expression.
fn tail_position(s: &str) -> Result<IfFrom, Error> {
    Ok(match parse(s) {
        //~^ manual_map_err
        Ok(g) => IfFrom::Group(g),
        Err(_) => return Err(Error),
    })
}

// A comment inside the match would be dropped by the rewrite, so the suggestion is emitted as
// `MaybeIncorrect` and rustfix leaves the code alone.
fn has_comment(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        //~^ manual_map_err
        // this comment would be lost
        Ok(v) => v * 2,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// `cfg` on an arm makes the rewrite unsound.
fn has_cfg(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        #[cfg(not(feature = "nope"))]
        Ok(v) => v * 2,
        #[cfg(feature = "nope")]
        Ok(v) => v * 3,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// An `if let` that is itself the `else` clause of an outer `if` needs braces around the
// replacement, since `else` must be followed by a block.
fn if_let_is_else_clause(s: &str, cond: bool) -> Result<u32, Error> {
    let v = if cond {
        0
    } else if let Ok(v) = parse(s) {
        //~^ manual_map_err
        v * 2
    } else {
        return Err(Error);
    };
    Ok(v)
}

// Used as a statement rather than for its value. A `match` needs no trailing semicolon there, but
// the `expr?` replacing it does. (winnow's `repeat` combinators look like this.)
fn as_statement(s: &str, acc: &mut Vec<u32>) -> Result<(), Error> {
    match parse(s) {
        //~^ manual_map_err
        Ok(v) => {
            acc.push(v);
        },
        Err(_) => return Err(Error),
    }

    acc.push(0);
    Ok(())
}

//
// Each case below shows a reason not to lint; some are paired with a contrasting nearby
// variant that does lint.
//

// A `bail!`-style macro expands to `return Err(..)`. Moving it into a `map_err` closure would
// return from the closure instead of the function, so this must not lint.
macro_rules! bail {
    ($msg:literal) => {
        return Err(Error)
    };
}

fn err_arm_is_bail_macro(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => v * 2,
        Err(_) => bail!("nope"),
    };
    Ok(v)
}

// The error value itself may come from a macro, as long as the `return` does not.
fn err_expr_from_macro(s: &str) -> Result<u32, String> {
    let v = match parse(s) {
        //~^ manual_map_err
        Ok(v) => v * 2,
        Err(_) => return Err(format!("nope: {s}")),
    };
    Ok(v)
}

// `Trait::assoc_fn(..)` gets its `Self` from the expected error type. `?` inserts a `From`
// conversion, so that expectation disappears and the call becomes ambiguous. (serde_yaml does
// this with `de::Error::invalid_type`.)
trait MakeErr {
    fn make(msg: &str) -> Self;
}
impl MakeErr for Error {
    fn make(_: &str) -> Self {
        Error
    }
}

fn err_from_trait_assoc_fn(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(_) => return Err(MakeErr::make("nope")),
    };
    Ok(v)
}

// The `.into()` target is pinned by the `return Err(..)` position. Behind `?`, which adds its own
// `From` conversion, the intermediate type is ambiguous (E0283). (uutils' tac does this.)
#[derive(Debug)]
struct WrappedErr(std::num::ParseIntError);
impl std::fmt::Display for WrappedErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl std::error::Error for WrappedErr {}

fn err_into_needs_return_context(s: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(e) => return Err(WrappedErr(e).into()),
    };
    Ok(v)
}

// `Err(Box::new(..))` relies on the unsize coercion `Box<ConcreteErr>` to `Box<dyn MyError>`,
// which only happens in the pinned `return` position. `?` needs a `From` impl instead, and no
// such impl exists. (uutils' ls does this with its `UError` trait.)
trait MyError {}
#[derive(Debug)]
struct ConcreteErr;
impl MyError for ConcreteErr {}

fn err_box_needs_unsize_coercion(s: &str) -> Result<u32, Box<dyn MyError>> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(_) => return Err(Box::new(ConcreteErr)),
    };
    Ok(v)
}

// `question_mark`'s job: the error is returned as is.
fn same_error(s: &str) -> Result<u32, std::num::ParseIntError> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    Ok(v)
}

// `question_mark`'s job: the error is only `into()`-converted.
fn error_into(s: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };
    Ok(v)
}

// `Option`, not `Result`.
fn on_option(o: Option<u32>) -> Result<u32, Error> {
    let v = match o {
        Some(v) => v,
        None => return Err(Error),
    };
    Ok(v)
}

// The `Ok` arm body escapes the enclosing function, so it cannot move into a closure.
fn ok_body_returns(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => {
            if v == 0 {
                return Err(Error);
            }
            v
        },
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The `Ok` arm body uses `?`, which also escapes.
fn ok_body_uses_question_mark(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => parse(s).map_err(|_| Error)? + v,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The `Ok` arm body breaks out of a loop.
#[allow(clippy::never_loop)]
fn ok_body_breaks(s: &str) -> Result<u32, Error> {
    loop {
        let v = match parse(s) {
            Ok(v) => break,
            Err(_) => return Err(Error),
        };
    }
    Ok(0)
}

// Matching on a reference: `map`/`map_err` need an owned `Result`.
fn on_reference(r: &Result<u32, Error>) -> Result<u32, Error> {
    let v = match r {
        Ok(v) => *v,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The error arm moves `guard`, but the `match` only did so on the diverging branch, so `guard` is
// still live afterwards. A `map_err` closure would move it unconditionally. (lock_api does this.)
struct Guard(u32);
impl Drop for Guard {
    fn drop(&mut self) {}
}

fn moved_local_used_after(guard: Guard) -> Result<u32, (Guard, String)> {
    let data = match parse("1") {
        Ok(data) => data,
        Err(e) => return Err((guard, e.to_string())),
    };
    drop(guard);
    Ok(data)
}

// Same shape, but nothing uses the moved local afterwards, so the rewrite is fine.
fn moved_local_not_used_after(guard: Guard) -> Result<u32, (Guard, String)> {
    let data = match parse("1") {
        //~^ manual_map_err
        Ok(data) => data,
        Err(e) => return Err((guard, e.to_string())),
    };
    Ok(data)
}

// Both arms move the same local, which would be a double move across two closures.
fn shared_move(s: &str, owned: String) -> Result<String, String> {
    let v = match parse(s) {
        Ok(_) => owned,
        Err(_) => return Err(owned),
    };
    Ok(v)
}

// Guards disable the rewrite.
fn with_guard(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) if v > 0 => v,
        Ok(_) => return Err(Error),
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// The `Err` arm does something other than `return Err(..)`.
fn err_arm_panics(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => v,
        Err(_) => panic!("nope"),
    };
    Ok(v)
}

// `by ref` binding.
fn ref_binding(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(ref v) => *v,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

// Inside a `try` block `?` targets a different type.
fn in_try_block(s: &str) -> Result<u32, Error> {
    let r: Result<u32, Error> = try {
        match parse(s) {
            Ok(v) => v,
            Err(_) => return Err(Error),
        }
    };
    r
}

// `?` is not usable in a const context.
const fn in_const_fn(r: Result<u32, Error>) -> Result<u32, Error> {
    let v = match r {
        Ok(v) => v,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

#[clippy::msrv = "1.12"]
fn msrv_too_low(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        Ok(v) => v * 2,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

#[clippy::msrv = "1.13"]
fn msrv_ok(s: &str) -> Result<u32, Error> {
    let v = match parse(s) {
        //~^ manual_map_err
        Ok(v) => v * 2,
        Err(_) => return Err(Error),
    };
    Ok(v)
}

fn main() {}
