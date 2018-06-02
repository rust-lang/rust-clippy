use std::rc::Rc;
use std::rc;
use std::sync::Arc;

#[warn(clippy, use_shared_from_slice)]
#[allow(unused_variables)]
fn main() {
    // Test constructing `Rc` directly from `vec!` macro.
    let bad_rc_vec_0: Rc<Vec<usize>> = Rc::new(vec!(1, 2, 3));
    let bad_rc_vec_00: rc::Rc<Vec<usize>> = rc::Rc::new(vec!(1, 2, 3));
    // Test constructing `Rc` from `Vec` variable.
    let example_vec: Vec<usize> = vec!(4, 5, 6);
    let bad_rc_vec_1: Rc<Vec<usize>> = Rc::new(example_vec);
    // Test constructing `Rc` with a `String`.
    let bad_rc_string_0: Rc<String> = Rc::new("test".to_string());
    // Test constructing `Rc` with a `String` variable.
    let example_string: String = "test".to_string();
    let bad_rc_string_1: Rc<String> = Rc::new(example_string);

    // Test constructing `Arc` from `vec!` macro.
    let bad_arc_vec_0: Arc<Vec<usize>> = Arc::new(vec!(1, 2, 3));
    // Test constructing `Arc` from `Vec` variable.
    let example_vec: Vec<usize> = vec!(4, 5, 6);
    let bad_arc_vec_1: Arc<Vec<usize>> = Arc::new(example_vec);
    // Test constructing `Arc` with a `String`.
    let bad_arc_string_0: Arc<String> = Arc::new("test".to_string());
    // Test constructing `Arc` with a `String` variable.
    let example_string: String = "test".to_string();
    let bad_arc_string_0: Arc<String> = Arc::new(example_string);

    // Test that using `.into()` doesn't cause suggestions.
    let good_rc_0: Rc<[usize]> = vec!(1, 2, 3).into();
    let example_vec: Vec<usize> = vec!(4, 5, 6);
    let good_rc_1: Rc<[usize]> = example_vec.into();
}
