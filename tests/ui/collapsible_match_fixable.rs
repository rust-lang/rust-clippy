#![warn(clippy::collapsible_match)]
#![allow(clippy::redundant_guards, clippy::single_match)]

fn issue16558() {
    let opt = Some(1);
    let _ = match opt {
        Some(s) => {
            if s == 1 { s } else { 1 }
            //~^ collapsible_match
        },
        _ => 1,
    };

    match opt {
        Some(s) => {
            (if s == 1 {
                //~^ collapsible_match
                todo!()
            })
        },
        _ => {},
    };

    let _ = match opt {
        Some(s) if s > 2 => {
            if s == 1 { s } else { 1 }
            //~^ collapsible_match
        },
        _ => 1,
    };
}

// https://github.com/rust-lang/rust-clippy/issues/16875
// lint still fires when only wildcard-like arms follow (fall-through is harmless)
fn issue16875(a: Option<&str>, b: i32) -> i32 {
    let mut res = 0;
    match a {
        Some(_) => {
            if b == 0 {
                //~^ collapsible_match
                res = 1;
            }
        },
        _ => {},
    }
    res
}

#[rustfmt::skip]
fn issue16716(n: u32) {
    match n {
        0 | 1 => if false { println!("hello world"); },
        //~^ collapsible_match
        _ => (),
    }

    let opt = Some(1);
    let _ = match opt {
        Some(s) => if s == 1 { s } else { 1 }
        //~^ collapsible_match
        _ => 1,
    };
}

#[rustfmt::skip]
fn issue16894() {
    let a = 5_u8;
    let b = a;
    _ = match a {
        11 => 0,
        13 => if a > b { 1 } else { 0 }, _ => 0,
        //~^ collapsible_match
    };

    _ = match a {
        11 => 0, 12 => if a == b { 1 } else { 0 }, _ => 0,
        //~^ collapsible_match
    };
}

// https://github.com/rust-lang/rust-clippy/issues/17427
// `collapsible_match` suggested wrong spans when the outer arm's block has a
// label, deleting the label's leading `'` instead of preserving it after `=>`.
fn issue17427() {
    let x: Result<Option<u8>, ()> = Ok(Some(1));
    match x {
        Ok(Some(_c)) => 'label: {
            if true {
                //~^ collapsible_match
                let Some(_y) = Some(0) else {
                    break 'label;
                };
            }
        },
        _ => (),
    }
}