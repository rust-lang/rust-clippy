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

fn lint_ref(struct1: &&Struct) {
    // this should suggest struct1.field1, NOT **struct1.field1
    let Struct { field1, .. } = **struct1;
}

macro_rules! mac {
    () => {
        Struct {
            field1: Some(1),
            field2: Some(2),
        }
    };
}

macro_rules! pat {
    ($id:ident) => {
        Struct { field1: $id, field2: _ }
    };
}

fn ok_macro() {
    let Struct { .. } = mac!();
    let pat!(a) = Struct {
        field1: None,
        field2: None,
    };
    let (pat!(a), _) = (mac!(), mac!());
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

fn main() {}
