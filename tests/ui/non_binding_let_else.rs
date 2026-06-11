#![warn(clippy::non_binding_let_else)]
#![allow(
    irrefutable_let_patterns,
    clippy::op_ref,
    clippy::partialeq_to_none,
    clippy::redundant_pattern_matching
)]

enum Status {
    Ok { a: i32, b: i32 },
    Err(i32),
}

#[derive(PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let tuple = (4, 7);
    let opt = Some(42);
    let pt = Point { x: 1, y: 2 };
    let status = Status::Err(1);

    let 0 = 1 else { return };
    //~^ non_binding_let_else

    let Some(42) = opt else { return };
    //~^ non_binding_let_else

    let None = opt else { return };
    //~^ non_binding_let_else

    let Status::Ok { .. } = status else { return };
    //~^ non_binding_let_else

    let Status::Err(_) = status else { panic!() };
    //~^ non_binding_let_else

    let Point { x: 1, y: 3 } = pt else { unreachable!() };
    //~^ non_binding_let_else

    // negative tests
    let val = 1 else { return };
    let (a, _) = tuple else { return };
    let Point { x: 1, y } = pt else { unreachable!() };
    let Status::Ok { a: aa, .. } = status else { panic!() };
}

macro_rules! let_else_return {
    ($pat:pat, $val:expr) => {
        let $pat = $val else { return };
    };
}

macro_rules! tuple_with_12 {
    ($($x:tt)*) => {
        ($($x)*, 12)
    };
}

fn test_macros() {
    let opt = Some(42);
    let status = Status::Err(1);

    let_else_return!(None, opt);
    //~^ non_binding_let_else

    let_else_return!(Some(0), opt);
    //~^ non_binding_let_else

    let_else_return!(Status::Ok { .. }, status);
    //~^ non_binding_let_else

    let_else_return!(Status::Ok { a: 1, .. }, status);
    //~^ non_binding_let_else

    let (11, 12) = tuple_with_12!(11) else { return };
    //~^ non_binding_let_else

    let tuple_with_12!(Some(_)) = (opt, 12) else { return };
    //~^ non_binding_let_else

    // negative tests
    let_else_return!(Some(n), opt);
    let_else_return!(Status::Ok { a, b: 1 }, status);
    let_else_return!(Status::Err(e), status);
    let (a, 100) = tuple_with_12!(None::<()>) else { return };
    let tuple_with_12!(Some(n)) = (opt, 12) else { return };
}
