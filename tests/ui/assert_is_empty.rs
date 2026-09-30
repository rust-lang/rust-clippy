#![warn(clippy::assert_is_empty)]
#![expect(clippy::needless_ifs, clippy::useless_vec)]
#![allow(clippy::const_is_empty)]

// `HashMap` is deliberately not imported and `BTreeMap` is only imported under an
// alias: the suggestion must name the collection by a path that resolves at the
// call site, not by its short name.
use std::collections::{BTreeMap as Tree, BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};
use std::hash::{BuildHasher, DefaultHasher};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct NotDebugOrd;

fn main() {
    let vec = vec![1, 2, 3];
    assert!(vec.is_empty());
    //~^ assert_is_empty
    assert!(!vec.is_empty());
    //~^ assert_is_empty
    debug_assert!(vec.is_empty());
    //~^ assert_is_empty
    debug_assert!(!vec.is_empty());
    //~^ assert_is_empty

    let vec_ref = &vec;
    assert!(vec_ref.is_empty());
    //~^ assert_is_empty
    assert!(!vec_ref.is_empty());
    //~^ assert_is_empty

    let vec_mut_ref = &mut vec![1, 2, 3];
    assert!(vec_mut_ref.is_empty());
    //~^ assert_is_empty

    let slice: &[i32] = &[1, 2, 3];
    assert!(slice.is_empty());
    //~^ assert_is_empty
    assert!(!slice.is_empty());
    //~^ assert_is_empty

    let array = [1, 2, 3];
    assert!(array.is_empty());
    //~^ assert_is_empty
    assert!(!array.is_empty());
    //~^ assert_is_empty

    let array_ref = &array;
    assert!(array_ref.is_empty());
    //~^ assert_is_empty

    // A bare `[]` does not infer for elements with several `PartialEq` impls,
    // so the empty array is typed.
    let string_array = [String::new()];
    assert!(string_array.is_empty());
    //~^ assert_is_empty
    let string_vec = vec![String::new()];
    assert!(string_vec.is_empty());
    //~^ assert_is_empty
    let string_vec_ref = &string_vec;
    assert!(string_vec_ref.is_empty());
    //~^ assert_is_empty
    let str_vec_ref = &vec!["foo"];
    assert!(str_vec_ref.is_empty());
    //~^ assert_is_empty

    let string = String::from("foo");
    assert!(string.is_empty());
    //~^ assert_is_empty
    assert!(!string.is_empty());
    //~^ assert_is_empty

    let str_ref = "foo";
    assert!(str_ref.is_empty());
    //~^ assert_is_empty
    assert!(!str_ref.is_empty());
    //~^ assert_is_empty

    let btreemap = Tree::<i32, i32>::new();
    assert!(btreemap.is_empty());
    //~^ assert_is_empty
    assert!(!btreemap.is_empty());
    //~^ assert_is_empty
    debug_assert!(btreemap.is_empty());
    //~^ assert_is_empty
    debug_assert!(!btreemap.is_empty());
    //~^ assert_is_empty

    let btreeset = BTreeSet::<i32>::new();
    assert!(btreeset.is_empty());
    //~^ assert_is_empty
    assert!(!btreeset.is_empty());
    //~^ assert_is_empty
    debug_assert!(btreeset.is_empty());
    //~^ assert_is_empty

    let hashmap = std::collections::HashMap::<i32, i32>::new();
    assert!(hashmap.is_empty());
    //~^ assert_is_empty
    assert!(!hashmap.is_empty());
    //~^ assert_is_empty
    debug_assert!(hashmap.is_empty());
    //~^ assert_is_empty

    let hashset = HashSet::<i32>::new();
    assert!(hashset.is_empty());
    //~^ assert_is_empty
    assert!(!hashset.is_empty());
    //~^ assert_is_empty
    debug_assert!(hashset.is_empty());
    //~^ assert_is_empty

    let linked_list = LinkedList::<i32>::new();
    assert!(linked_list.is_empty());
    //~^ assert_is_empty
    assert!(!linked_list.is_empty());
    //~^ assert_is_empty
    debug_assert!(linked_list.is_empty());
    //~^ assert_is_empty

    let vec_deque = VecDeque::<i32>::new();
    assert!(vec_deque.is_empty());
    //~^ assert_is_empty
    assert!(!vec_deque.is_empty());
    //~^ assert_is_empty
    debug_assert!(vec_deque.is_empty());
    //~^ assert_is_empty

    // `&C` does not compare with `C`, so borrowed collections are dereferenced.
    let hashmap_ref = &hashmap;
    assert!(hashmap_ref.is_empty());
    //~^ assert_is_empty
    assert!(!hashmap_ref.is_empty());
    //~^ assert_is_empty

    let vec_deque_mut_ref = &mut VecDeque::<i32>::new();
    assert!(vec_deque_mut_ref.is_empty());
    //~^ assert_is_empty
    assert!(!vec_deque_mut_ref.is_empty());
    //~^ assert_is_empty

    let hashmap_ref_ref = &hashmap_ref;
    assert!(hashmap_ref_ref.is_empty());
    //~^ assert_is_empty

    // Collection bounds are checked on the whole type: `HashMap<K, V>: PartialEq`
    // needs `K: Eq + Hash` and `V: PartialEq`, and `Debug` needs both `Debug`.
    let string_map = std::collections::HashMap::<String, Vec<u8>>::new();
    assert!(string_map.is_empty());
    //~^ assert_is_empty

    // `BinaryHeap` implements no `PartialEq`, so it is compared through
    // `BinaryHeap::as_slice`. The empty array must be typed: a bare `[]` leaves
    // the element type ambiguous for elements like `String`.
    let binary_heap = BinaryHeap::<i32>::new();
    assert!(binary_heap.is_empty());
    //~^ assert_is_empty
    assert!(!binary_heap.is_empty());
    //~^ assert_is_empty
    debug_assert!(binary_heap.is_empty());
    //~^ assert_is_empty

    let string_heap = BinaryHeap::<String>::new();
    assert!(string_heap.is_empty());
    //~^ assert_is_empty

    let binary_heap_ref = &binary_heap;
    assert!(binary_heap_ref.is_empty());
    //~^ assert_is_empty

    // Don't lint: `BinaryHeap<T>` needs `T: Debug + PartialEq` to be compared and printed.
    let not_debug_heap = BinaryHeap::<NotDebugOrd>::new();
    assert!(not_debug_heap.is_empty());

    // Don't lint: `HashMap::default()` requires the hasher to implement
    // `Default`, so a map with a non-`Default` hasher has no compiling
    // suggestion.
    struct NotDefaultHasher;
    impl BuildHasher for NotDefaultHasher {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> DefaultHasher {
            DefaultHasher::new()
        }
    }
    let not_default_hasher_map = std::collections::HashMap::<i32, i32, _>::with_hasher(NotDefaultHasher);
    assert!(not_default_hasher_map.is_empty());
    assert!(!not_default_hasher_map.is_empty());

    // A hasher that implements `Default` is fine: `HashMap::default()` compiles
    // for it, unlike `HashMap::new()`.
    #[derive(Default)]
    struct DefaultHasherBuilder;
    impl BuildHasher for DefaultHasherBuilder {
        type Hasher = DefaultHasher;
        fn build_hasher(&self) -> DefaultHasher {
            DefaultHasher::new()
        }
    }
    let default_hasher_map = std::collections::HashMap::<i32, i32, DefaultHasherBuilder>::default();
    assert!(default_hasher_map.is_empty());
    //~^ assert_is_empty

    // Don't lint: has assert message.
    assert!(vec.is_empty(), "items should be empty");
    assert!(!vec.is_empty(), "items should not be empty");
    assert!(vec.is_empty(), "unexpected values: {vec:?}");
    assert!(!vec.is_empty(), "unexpected values: {vec:?}");

    // Common chained assertion shape. The first assertion can hide the value
    // that the second assertion would otherwise help diagnose.
    let items = vec!["baz"];
    assert!(!items.is_empty());
    //~^ assert_is_empty
    assert_eq!(items[0], "bar");

    // Don't lint: the outer `assert!` is written here, but the condition comes
    // from a macro expansion. Rewriting the expanded condition span would make
    // the suggestion point at generated code rather than source code.
    macro_rules! is_empty {
        ($value:expr) => {
            $value.is_empty()
        };
    }
    assert!(is_empty!(vec));

    // Don't lint: the `assert!` itself comes from a macro expansion. The lint
    // only rewrites assert calls that are present at the call site.
    macro_rules! assert_empty {
        ($value:expr) => {
            assert!($value.is_empty())
        };
    }
    assert_empty!(vec);

    // Don't lint: not an assert macro.
    if vec.is_empty() {}
    if !vec.is_empty() {}

    // Don't lint: assert_eq! would require Debug and PartialEq for the element type.
    struct NotDebugOrPartialEq;
    let not_debug_or_partial_eq = vec![NotDebugOrPartialEq];
    assert!(not_debug_or_partial_eq.is_empty());

    #[derive(Debug)]
    struct NotPartialEq;
    let not_partial_eq = vec![NotPartialEq];
    assert!(not_partial_eq.is_empty());

    #[derive(PartialEq)]
    struct NotDebug;
    let not_debug = vec![NotDebug];
    assert!(not_debug.is_empty());

    // Don't lint: `HashMap<K, V>: Debug + PartialEq` requires `V: Debug + PartialEq`.
    let not_partial_eq_map = std::collections::HashMap::<i32, NotPartialEq>::new();
    assert!(not_partial_eq_map.is_empty());

    let not_debug_map = std::collections::HashMap::<i32, NotDebug>::new();
    assert!(not_debug_map.is_empty());

    let not_debug_set = BTreeSet::<NotDebug>::new();
    assert!(not_debug_set.is_empty());
}
