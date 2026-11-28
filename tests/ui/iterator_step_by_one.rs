#![expect(clippy::useless_vec)]
#[warn(clippy::iterator_step_by_one)]
fn main() {
    let _ = vec!["A", "B", "B"].iter().step_by(1);
    //~^ iterator_step_by_one

    let _ = "XXX".chars().step_by(1);
    //~^ iterator_step_by_one

    let _ = (0..1).step_by(1);
    //~^ iterator_step_by_one

    // No error, not an iterator.
    let y = NotIterator;
    y.step_by(1);

    // No warning for  step != 1
    let _ = (0..1).step_by(3);

    let _ = (1..).step_by(1);
    //~^ iterator_step_by_one

    let _ = (1..=2).step_by(1);
    //~^ iterator_step_by_one

    let x = 0..1;
    let _ = x.step_by(1);
    //~^ iterator_step_by_one

    // check const eval
    let v1 = vec![1, 2, 3];
    let _ = v1.iter().step_by(4 / 3);
    //~^ iterator_step_by_one
}

struct NotIterator;
impl NotIterator {
    fn step_by(&self, _: u32) {}
}
