//@no-rustfix: requires manual changes
#![warn(clippy::double_ended_iterator_last)]

// Should not be linted because applying the lint would move the original iterator. This can only be
// linted if the iterator is used thereafter.
fn main() {
    let mut index = [true, true, false, false, false, true].iter();
    let subindex = (index.by_ref().take(3), 42);
    let _ = subindex.0.last();
    let _ = index.next();
}
