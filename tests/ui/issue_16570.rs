#![allow(clippy::collapsible_else_if, stable_features)]
#![allow(dead_code)]

fn main() {
    let opt = Some(Some(1));

    // This should trigger collapsible_match
    let _ = if true && let Some(inner) = opt {
        match inner {
            //~^ ERROR: this `match` can be collapsed into the outer `if let`
            Some(_) => 0,
            _ => 1,
        }
    } else {
        1
    };

    // Another case with multiple let chains
    let _ = if let Some(inner_opt) = opt
        && let Some(inner) = inner_opt
    {
        match inner {
            //~^ ERROR: this `match` can be collapsed into the outer `if let`
            1 => 0,
            _ => 1,
        }
    } else {
        1
    };
}
