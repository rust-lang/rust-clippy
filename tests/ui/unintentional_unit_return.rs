#![warn(clippy::unintentional_unit_return)]
#![feature(is_sorted)]

struct Struct {
    field: isize,
}

fn double(i: isize) -> isize {
    i * 2
}

/*
fn fn_with_closure<T, F, K>(mut v: Vec<T>, f: F) where
    F: FnMut(&T) -> K,
    K: Ord {
    v.sort_by_key(f)
}
*/

fn main() {
    let mut structs = vec![Struct { field: 2 }, Struct { field: 1 }];
    structs.sort_by_key(|s| {
        double(s.field);
    });
    structs.sort_by_key(|s| double(s.field));
    structs.is_sorted_by_key(|s| {
        double(s.field);
    });
}
