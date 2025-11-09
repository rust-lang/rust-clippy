//@no-rustfix
#![warn(clippy::rwlock_atomic, clippy::rwlock_integer)]

use std::sync::RwLock;

fn none_issue_yet() {
    static MTX: RwLock<u32> = RwLock::new(0);
    //~^ rwlock_integer

    // unfixable because we don't fix this `write`
    let mut guard = MTX.write().unwrap();
    *guard += 1;
}
