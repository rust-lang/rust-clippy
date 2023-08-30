#![allow(unused)]
#![warn(
    clippy::all,
    clippy::style,
    clippy::mem_replace_option_with_none,
    clippy::mem_replace_with_default
)]

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::mem;

fn replace_option_with_none() {
    let mut an_option = Some(1);
    let _ = mem::replace(&mut an_option, None);
    //~^ ERROR: replacing an `Option` with `None`
    //~| NOTE: `-D clippy::mem-replace-option-with-none` implied by `-D warnings`
    let an_option = &mut Some(1);
    let _ = mem::replace(an_option, None);
    //~^ ERROR: replacing an `Option` with `None`
}

fn replace_with_default() {
    let mut s = String::from("foo");
    let _ = std::mem::replace(&mut s, String::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
    //~| NOTE: `-D clippy::mem-replace-with-default` implied by `-D warnings`

    let s = &mut String::from("foo");
    let _ = std::mem::replace(s, String::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
    let _ = std::mem::replace(s, Default::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut v = vec![123];
    let _ = std::mem::replace(&mut v, Vec::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
    let _ = std::mem::replace(&mut v, Default::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
    let _ = std::mem::replace(&mut v, Vec::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
    let _ = std::mem::replace(&mut v, vec![]);
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut hash_map: HashMap<i32, i32> = HashMap::new();
    let _ = std::mem::replace(&mut hash_map, HashMap::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut btree_map: BTreeMap<i32, i32> = BTreeMap::new();
    let _ = std::mem::replace(&mut btree_map, BTreeMap::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut vd: VecDeque<i32> = VecDeque::new();
    let _ = std::mem::replace(&mut vd, VecDeque::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut hash_set: HashSet<&str> = HashSet::new();
    let _ = std::mem::replace(&mut hash_set, HashSet::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut btree_set: BTreeSet<&str> = BTreeSet::new();
    let _ = std::mem::replace(&mut btree_set, BTreeSet::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut list: LinkedList<i32> = LinkedList::new();
    let _ = std::mem::replace(&mut list, LinkedList::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut binary_heap: BinaryHeap<i32> = BinaryHeap::new();
    let _ = std::mem::replace(&mut binary_heap, BinaryHeap::new());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut tuple = (vec![1, 2], BinaryHeap::<i32>::new());
    let _ = std::mem::replace(&mut tuple, (vec![], BinaryHeap::new()));
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut refstr = "hello";
    let _ = std::mem::replace(&mut refstr, "");
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin

    let mut slice: &[i32] = &[1, 2, 3];
    let _ = std::mem::replace(&mut slice, &[]);
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
}

// lint is disabled for primitives because in this case `take`
// has no clear benefit over `replace` and sometimes is harder to read
fn dont_lint_primitive() {
    let mut pbool = true;
    let _ = std::mem::replace(&mut pbool, false);

    let mut pint = 5;
    let _ = std::mem::replace(&mut pint, 0);
}

fn main() {
    replace_option_with_none();
    replace_with_default();
    dont_lint_primitive();
}

#[clippy::msrv = "1.39"]
fn msrv_1_39() {
    let mut s = String::from("foo");
    let _ = std::mem::replace(&mut s, String::default());
}

#[clippy::msrv = "1.40"]
fn msrv_1_40() {
    let mut s = String::from("foo");
    let _ = std::mem::replace(&mut s, String::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
}

fn issue9824() {
    struct Foo<'a>(Option<&'a str>);
    impl<'a> std::ops::Deref for Foo<'a> {
        type Target = Option<&'a str>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<'a> std::ops::DerefMut for Foo<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    struct Bar {
        opt: Option<u8>,
        val: String,
    }

    let mut f = Foo(Some("foo"));
    let mut b = Bar {
        opt: Some(1),
        val: String::from("bar"),
    };

    // replace option with none
    let _ = std::mem::replace(&mut f.0, None);
    //~^ ERROR: replacing an `Option` with `None`
    let _ = std::mem::replace(&mut *f, None);
    //~^ ERROR: replacing an `Option` with `None`
    let _ = std::mem::replace(&mut b.opt, None);
    //~^ ERROR: replacing an `Option` with `None`
    // replace with default
    let _ = std::mem::replace(&mut b.val, String::default());
    //~^ ERROR: replacing a value of type `T` with `T::default()` is better expressed usin
}
