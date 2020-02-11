#![warn(clippy::type_complexity)]

use std::iter::{Filter, Map};
use std::vec::IntoIter;

struct S;

impl IntoIterator for S {
    type Item = i128;
    // Associated types should not trigger a type_complexity message.
    type IntoIter = Filter<Map<IntoIter<i128>, fn(i128) -> i128>, fn(&i128) -> bool>;

    fn into_iter(self) -> Self::IntoIter {
        fn m(a: i128) -> i128 {
            a
        }
        fn p(_: &i128) -> bool {
            true
        }
        vec![1i128, 2, 3].into_iter().map(m as fn(_) -> _).filter(p)
    }
}

fn main() {}
