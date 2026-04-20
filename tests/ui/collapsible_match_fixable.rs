#![warn(clippy::collapsible_match)]
#![allow(clippy::single_match, clippy::redundant_guards)]

// issue #16860: collapsible_match suggestion causes non-exhaustive pattern error
// when the else arm is `None` (not a true wildcard)
fn issue16860() -> Option<i32> {
    let x: Option<i32> = Some(1);
    match x {
        None => {},
        Some(v) => {
            if v > 0 {
                //~^ collapsible_match
                return Some(v);
            }
        },
    }
    None
}

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
