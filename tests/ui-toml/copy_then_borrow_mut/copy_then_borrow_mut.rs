#[test]
fn in_test() {
    let _ = &mut { 42 }; // Do not lint
}

fn main() {
    let _ = &mut { 42 }; //~ ERROR: mutable borrow
}
