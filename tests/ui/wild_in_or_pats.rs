#![warn(clippy::wildcard_in_or_patterns)]

fn main() {
    match "foo" {
        "a" => {
            dbg!("matched a");
        },
        "bar" | _ => {
        //~^ wildcard_in_or_patterns
            dbg!("matched (bar or) wild");
        },
    };
    match "foo" {
        "a" => {
            dbg!("matched a");
        },
        "bar" | "bar2" | _ => {
        //~^ wildcard_in_or_patterns
            dbg!("matched (bar or bar2 or) wild");
        },
    };
    match "foo" {
        "a" => {
            dbg!("matched a");
        },
        _ | "bar" | _ => {
        //~^ wildcard_in_or_patterns
            dbg!("matched (bar or) wild");
        },
    };
    match "foo" {
        "a" => {
            dbg!("matched a");
        },
        _ | "bar" => {
        //~^ wildcard_in_or_patterns
            dbg!("matched (bar or) wild");
        },
    };
}
