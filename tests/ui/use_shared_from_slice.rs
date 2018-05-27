use std::rc::Rc;

fn main() {
    let bad_ref: Rc<Vec<usize>> = Rc::new(vec!(1, 2, 3));

    let good_ref: Rc<[usize]> = vec!(1, 2, 3).into();
}
