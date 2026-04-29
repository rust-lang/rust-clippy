#![warn(clippy::non_binding_let_patterns)]
#![allow(
    irrefutable_let_patterns,
    clippy::eq_op,
    clippy::needless_ifs,
    clippy::never_loop,
    clippy::partialeq_to_none,
    clippy::redundant_pattern_matching,
    clippy::while_immutable_condition
)]

#[derive(PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

enum Status {
    Ok,
    Err(i32),
}

fn main() {
    let tuple = (4, 7);
    let opt = Some(42);
    let pt = Point { x: 1, y: 2 };
    let status = Status::Ok;

    // --- let-else cases ---

    let 0 = 1 else { return };
    //~^ non_binding_let_patterns

    let Some(42) = opt else { return };
    //~^ non_binding_let_patterns

    let None = opt else { return };
    //~^ non_binding_let_patterns

    let Status::Err(_) = status else { return };
    //~^ non_binding_let_patterns

    // --- if-let cases ---

    if let 42 = 42 {}
    //~^ non_binding_let_patterns

    if let (6, 7) = tuple {}
    //~^ non_binding_let_patterns

    if let Point { x: 1, y: 2 } = pt {}
    //~^ non_binding_let_patterns

    if let Status::Ok = status {}
    //~^ non_binding_let_patterns

    // --- while-let cases ---

    let mut x = 0;
    while let 0 = x {
        //~^ non_binding_let_patterns
        x += 1;
    }

    while let None = opt {
        //~^ non_binding_let_patterns
        break;
    }

    // --- negative tests ---

    let val = 1 else { return };
    let (a, b) = tuple else { return };
    if let Some(x) = opt {}
    if let Some(Some(inner)) = Some(Some(10)) {}
    if let Point { x, y: 2 } = pt {}
    while let Status::Err(e) = status {
        break;
    }
}
