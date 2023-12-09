#![warn(clippy::manual_rfold)]

struct FakeIter(std::ops::Range<i32>);

impl FakeIter {
    fn rev(self) -> Self {
        self
    }

    fn next(&self) {}
}

impl DoubleEndedIterator for FakeIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl Iterator for FakeIter {
    type Item = i32;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

fn main() {
    // should not lint
    // FakeIter(0..10).rev().next();

    // should lint
    let _ = (1..10).rev().fold(50, |acc, x| acc / x);
    let _ = "something".bytes().rev().fold(50, |acc, x| acc / x);
}
