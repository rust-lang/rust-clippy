#![warn(clippy::significant_drop_in_scrutinee)]
//@no-rustfix

use std::sync::Mutex;

fn should_trigger_lint_in_while_let() {
    let mutex = Mutex::new(vec![1]);

    while let Some(val) = mutex.lock().unwrap().pop() {
        //~^ ERROR: temporary with significant `Drop` in `while let` scrutinee will live until the
        //~| NOTE: this might lead to deadlocks or other unexpected behavior
        println!("{}", val);
    }
}
