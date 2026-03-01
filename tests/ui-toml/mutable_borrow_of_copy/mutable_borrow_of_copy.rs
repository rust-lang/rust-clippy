#[test]
fn in_test() {
    let mut a = [10; 2];
    let _ = &mut { a }; // Do not lint
}

fn main() {
    let mut a = [10; 2];
    let _ = &mut { a }; //~ mutable_borrow_of_copy
}
