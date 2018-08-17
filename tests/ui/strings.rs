


#[warn(string_add)]
#[allow(string_add_assign)]
fn add_only() { // ignores assignment distinction
    let mut x = "".to_owned();

    for _ in 1..3 {
        x = x + ".";
    }

    let y = "".to_owned();
    let z = y + "...";

    assert_eq!(&x, &z);
}

#[warn(string_add_assign)]
fn add_assign_only() {
    let mut x = "".to_owned();

    for _ in 1..3 {
        x = x + ".";
    }

    let y = "".to_owned();
    let z = y + "...";

    assert_eq!(&x, &z);
}

#[warn(string_add, string_add_assign)]
fn both() {
    let mut x = "".to_owned();

    for _ in 1..3 {
        x = x + ".";
    }

    let y = "".to_owned();
    let z = y + "...";

    assert_eq!(&x, &z);
}

#[allow(dead_code, unused_variables)]
#[warn(string_lit_as_bytes)]
fn str_lit_as_bytes() {
    let bs = "hello there".as_bytes();

    // no warning, because this cannot be written as a byte string literal:
    let ubs = "☃".as_bytes();

    let strify = stringify!(foobar).as_bytes();
}

#[warn(string_repeat_once)]
fn str_repeat() {
    let a = "this shouldn't work".repeat(1);
    let b = String::from("neither should this");
    let c = b.repeat(1);
    let d = "this works".repeat(2);
}

fn main() {
    add_only();
    add_assign_only();
    both();
    str_repeat();

    // the add is only caught for `String`
    let mut x = 1;
    ; x = x + 1;
    assert_eq!(2, x);
}
