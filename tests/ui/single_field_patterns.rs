#![warn(clippy::single_field_patterns)]
struct Struct {
    field1: Option<i32>,
    field2: Option<i32>,
}

fn lint_struct(struct1: Struct) {
    let Struct { field1, .. } = struct1;
    let Struct { field1, field2: _ } = struct1;
    match struct1 {
        Struct { field1: Some(n), .. } if n >= 50 => {},
        Struct { field1: None, .. } => {},
        _ => {},
    }
    match struct1 {
        Struct { field1: Some(n), .. } if n >= 50 => {},
        Struct { .. } => {},
        _ => {},
    }
    match struct1 {
        Struct { field1: Some(1), .. } => {},
        Struct { field1: Some(2), .. } => {},
        Struct { field1: None, .. } => {},
        _ => {},
    }
    match struct1 {
        Struct {
            field1: Some(_) | None, ..
        } => {},
    }
    while let Struct { field1: Some(5), .. } = struct1 {}
}

struct Tuple(Option<i32>, Option<i32>);

fn lint_tuple_struct(tuple: Tuple) {
    if let Tuple(Some(x), _) = tuple {}
    if let Tuple(_, Some(y)) = tuple {}
    match tuple {
        Tuple(Some(1), ..) => {},
        Tuple(Some(2), ..) => {},
        Tuple(a, ..) => {},
    }
    match tuple {
        Tuple(Some(1) | Some(42) | Some(6082), ..) => {},
        Tuple(a, ..) => {},
    }
}

fn lint_tuple(tuple: (Option<i32>, Option<i32>)) {
    if let (Some(z), _) = tuple {}
    if let (_, Some(n)) = tuple {}
    match tuple {
        (Some(1), ..) => {},
        (Some(2), ..) => {},
        (a, ..) => {},
    }
    match tuple {
        (Some(1) | Some(42) | Some(6082), ..) => {},
        (a, ..) => {},
    }
}

fn lint_array(array: [i32; 3]) {
    match array {
        [1 | 2, ..] => {},
        [x @ 3, ..] => {},
        [r @ 20..=65, ..] => {},
        [e, ..] => {},
    }
    if let [5, ..] = array {}
}

fn ok_struct(struct1: Struct) {
    let _ = struct1;
    let Struct { field1, field2, .. } = struct1;
    let Struct { field1, field2 } = struct1;
    match struct1 {
        Struct {
            field1: Some(1),
            field2: _,
        } => {},
        Struct {
            field1: _,
            field2: Some(1),
        } => {},
        _ => {},
    }
    match struct1 {
        Struct { field1: Some(1), .. } => {},
        Struct {
            field1: _,
            field2: Some(1),
        } => {},
        _ => {},
    }
    let s @ Struct { field1, .. } = struct1;
}

fn ok_tuple_struct(tuple: Tuple) {
    if let Tuple(Some(1), two) = tuple {}
    if let Tuple(one, Some(1)) = tuple {}
    match tuple {
        Tuple(Some(1), _) => {},
        Tuple(_, Some(1)) => {},
        _ => {},
    }
    match tuple {
        Tuple(Some(1), ..) => {},
        Tuple(.., Some(1)) => {},
        _ => {},
    }
    match tuple {
        Tuple(.., Some(1)) => {},
        Tuple(.., Some(2)) => {},
        Tuple(.., None) => {},
        _ => {},
    }
    let t @ Tuple(_, two) = tuple;
}

fn ok_tuple(tuple: (Option<i32>, Option<i32>)) {
    if let (None, two) = tuple {}
    if let (one, Some(1)) = tuple {}
    match tuple {
        (Some(1), _) => {},
        (_, Some(1)) => {},
        _ => {},
    }
    match tuple {
        (.., Some(1)) => {},
        (.., Some(2)) => {},
        (.., None) => {},
        _ => {},
    }
    match tuple {
        (Some(1), ..) => {},
        (.., Some(1)) => {},
        _ => {},
    }
    let t @ (one, two) = tuple;
}

fn ok_array(array: [i32; 555]) {
    if let [.., 209, _] = array {}
}

fn ok_slice(slice: &[i32]) {
    // slices may panic if indexed, but will never panic
    // in a pattern since they're forced to check if they're empty
    match slice {
        [.., x] => {},
        [] => {},
    }
    if let [x, ..] = slice {}
}

enum Enum {
    Enum(Option<i32>, Option<i32>),
    OtherEnum,
}

fn ok_enum(enum1: Enum) {
    // Should never touch enums for this lint
    // since there's no way to access a field directly
    if let Enum::Enum(one, _) = enum1 {}
    match enum1 {
        Enum::Enum(one, ..) => {},
        Enum::Enum(_, _) => {},
        Enum::OtherEnum => {},
    }
}

fn main() {}
