#![warn(clippy::unnecessary_paren_in_negated_if)]

fn main() {
    let condition = true;

    // These should trigger the lint
    if !(condition) {
        println!("Negated condition with unnecessary parentheses");
    }

    if !(condition) {
        println!("Negated condition with double unnecessary parentheses");
    }

    // These should not trigger the lint
    if !condition {
        println!("Negated condition without parentheses");
    }

    if !(condition && true) {
        println!("Negated condition with necessary parentheses");
    }

    if !(condition || false) {
        println!("Another negated condition with necessary parentheses");
    }
}
