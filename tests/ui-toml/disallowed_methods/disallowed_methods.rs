#![warn(clippy::disallowed_methods)]

macro_rules! call_ok {
    ($e:expr) => {
        $e.ok()
    };
}

fn main() {
    let r: Result<i32, ()> = Ok(1);

    // This call is inside a `macro_rules!` expansion, so it should not be linted:
    let _ = call_ok!(r);

    // Direct user code should still be linted:
    let _ = r.ok();
    //~^ disallowed_methods
}
