#![warn(clippy::rwlock_integer)]
#![warn(clippy::rwlock_atomic)]
#![allow(clippy::borrow_as_ptr)]

use std::sync::RwLock;

fn main() {
    let _ = RwLock::new(true);
    //~^ rwlock_atomic

    let _ = RwLock::new(5usize);
    //~^ rwlock_atomic

    let _ = RwLock::new(9isize);
    //~^ rwlock_atomic

    let mut x = 4u32;
    // `AtomicPtr` only accepts `*mut T`, so this should not lint
    let _ = RwLock::new(&x as *const u32);

    let _ = RwLock::new(&mut x as *mut u32);
    //~^ rwlock_atomic

    let _ = RwLock::new(0u32);
    //~^ rwlock_integer

    let _ = RwLock::new(0i32);
    //~^ rwlock_integer

    let _ = RwLock::new(0f32); // there are no float atomics, so this should not lint
    let _ = RwLock::new(0u8);
    //~^ rwlock_integer

    let _ = RwLock::new(0i16);
    //~^ rwlock_integer

    let _x: RwLock<i8> = RwLock::new(0);
    //~^ rwlock_integer

    const X: i64 = 0;
    let _ = RwLock::new(X);
    //~^ rwlock_integer

    // there are no 128 atomics, so these two should not lint
    {
        let _ = RwLock::new(0u128);
        let _x: RwLock<i128> = RwLock::new(0);
    }
}
