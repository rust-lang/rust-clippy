#![feature(plugin)]
#![plugin(clippy)]

#![deny(block_in_if_condition_expr)]
#![deny(block_in_if_condition_stmt)]

fn condition_has_block_with_single_expression() -> i32 {
    if { true } { //~ERROR remove braces
        6
    } else {
        10
    }
}

fn condition_has_block() -> i32 {
    if { //~ERROR move block higher
        let x = 3;
        x == 3
    } {
        6
    } else {
        10
    }
}

fn condition_is_normal() -> i32 {
    let x = 3;
    if true && x == 3 {
        6
    } else {
        10
    }
}

// need a main anyway, use it get rid of unused warnings too
fn main() {
    assert!(condition_has_block_with_single_expression() == 6);
    assert!(condition_has_block() == 6);
    assert!(condition_is_normal() == 6);
}
