#![warn(clippy::option_if_let_else)]

fn bad1(string: Option<&str>) -> (bool, &str) {
    if let Some(x) = string {
        (true, x)
    } else {
        (false, "hello")
    }
}

fn main() {
    let optional = Some(5);
    let _ = if let Some(x) = optional {
        x + 2
    } else {
        5
    };
    let _ = bad1(None);
}
