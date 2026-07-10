#![warn(clippy::iter_missing_exact_size)]
#![allow(clippy::needless_return)]
#![allow(unreachable_code)]

use std::ops::Range;

// Struct field
struct StringRepeater1 {
    original: String,
    range: Range<usize>,
}

impl Iterator for StringRepeater1 {
    //~^ iter_missing_exact_size
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|i| self.original.repeat(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

// Tuple-like struct
struct StringRepeater2(String, Range<usize>);

impl Iterator for StringRepeater2 {
    //~^ iter_missing_exact_size
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|i| self.0.repeat(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.1.size_hint()
    }
}

// Uses `return _;` rather than a trailing expression
struct StringRepeater3(String, Range<usize>);

impl Iterator for StringRepeater3 {
    //~^ iter_missing_exact_size
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|i| self.0.repeat(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.1.size_hint();
    }
}

// Uses `return _;` followed by a dead-code trailing expression
struct StringRepeater4(String, Range<usize>);

impl Iterator for StringRepeater4 {
    //~^ iter_missing_exact_size
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|i| self.0.repeat(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        return self.1.size_hint();
        (5, Some(5))
    }
}

// Already marked as an ExactSizeIterator
struct StringRepeater5(String, Range<usize>);

impl Iterator for StringRepeater5 {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|i| self.0.repeat(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.1.size_hint()
    }
}
impl ExactSizeIterator for StringRepeater5 {}

// Delegates but to a non-ExactSizeIterator iterator
struct MyCollection {
    elements: Vec<u8>,
}
impl MyCollection {
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.elements.len(), Some(self.elements.len()))
    }
}

struct MyCollectionIter {
    inner: MyCollection,
}

impl Iterator for MyCollectionIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.elements.pop()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// Delegates to an ExactSizeIterator but not an object field
struct EmptyWithHint {}

fn range_provider() -> (Range<usize>,) {
    (0..5,)
}

impl Iterator for EmptyWithHint {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        range_provider().0.size_hint()
    }
}

fn main() {
    let repeater = StringRepeater1 {
        original: "Foo".to_string(),
        range: 1..5,
    };
    for value in repeater {
        println!("{value}");
    }

    let repeater = StringRepeater2("Bar".to_string(), 1..5);
    for value in repeater {
        println!("{value}");
    }

    let repeater = StringRepeater3("Bar".to_string(), 1..5);
    for value in repeater {
        println!("{value}");
    }

    let repeater = StringRepeater4("Bar".to_string(), 1..5);
    for value in repeater {
        println!("{value}");
    }

    let repeater = StringRepeater5("Bar".to_string(), 1..5);
    for value in repeater {
        println!("{value}");
    }

    let collection = MyCollectionIter {
        inner: MyCollection {
            elements: vec![3, 2, 1],
        },
    };
    for value in collection {
        println!("{value}");
    }

    let mut empty_provider = EmptyWithHint {};
    assert_eq!(None, empty_provider.next());
}
