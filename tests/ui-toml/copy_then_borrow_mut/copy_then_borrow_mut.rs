#[test]
fn in_test() {
    let a = [10; 2];
    let _ = &mut { a }; // Do not lint
}

fn main() {
    let a = [10; 2];
    let _ = &mut { a }; //~ ERROR: mutable borrow
}
