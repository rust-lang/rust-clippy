#![warn(clippy::option_if_let_else)]

fn main() {
    let optional = Some(5);
    if let Some(x) = optional {
        x+2
    } else {
        5
    }
}
