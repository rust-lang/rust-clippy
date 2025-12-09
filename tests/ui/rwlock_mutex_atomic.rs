#![warn(clippy::rwlock_integer)]
#![warn(clippy::rwlock_atomic)]
#![warn(clippy::mutex_integer)]
#![warn(clippy::mutex_atomic)]
#![allow(clippy::borrow_as_ptr)]

use std::sync::{Mutex, RwLock};

fn main() {
    let _ = RwLock::new(true);
    //~^ rwlock_atomic
    let _ = Mutex::new(true);
    //~^ mutex_atomic

    let _ = RwLock::new(5usize);
    //~^ rwlock_atomic
    let _ = Mutex::new(5usize);
    //~^ mutex_atomic

    let _ = RwLock::new(9isize);
    //~^ rwlock_atomic
    let _ = Mutex::new(9isize);
    //~^ mutex_atomic

    let mut x = 4u32;
    // `AtomicPtr` only accepts `*mut T`, so these should not lint
    let _ = RwLock::new(&x as *const u32);
    let _ = Mutex::new(&x as *const u32);

    let _ = RwLock::new(&mut x as *mut u32);
    //~^ rwlock_atomic
    let _ = Mutex::new(&mut x as *mut u32);
    //~^ mutex_atomic

    let _ = RwLock::new(0u32);
    //~^ rwlock_integer
    let _ = Mutex::new(0u32);
    //~^ mutex_integer

    let _ = RwLock::new(0i32);
    //~^ rwlock_integer
    let _ = Mutex::new(0i32);
    //~^ mutex_integer

    let _ = RwLock::new(0f32); // there are no float atomics, so this should not lint
    let _ = RwLock::new(0u8);
    //~^ rwlock_integer
    let _ = Mutex::new(0f32); // there are no float atomics, so this should not lint
    let _ = Mutex::new(0u8);
    //~^ mutex_integer

    let _ = RwLock::new(0i16);
    //~^ rwlock_integer
    let _ = Mutex::new(0i16);
    //~^ mutex_integer

    let _x: RwLock<i8> = RwLock::new(0);
    //~^ rwlock_integer
    let _x: Mutex<i8> = Mutex::new(0);
    //~^ mutex_integer

    const X: i64 = 0;
    let _ = RwLock::new(X);
    //~^ rwlock_integer
    let _ = Mutex::new(X);
    //~^ mutex_integer

    // there are no 128 atomics, so these four should not lint
    {
        let _ = RwLock::new(0u128);
        let _ = Mutex::new(0u128);
        let _x: RwLock<i128> = RwLock::new(0);
        let _x: Mutex<i128> = Mutex::new(0);
    }
}

// don't lint on _use_, only declaration
fn issue13378() {
    static MTX: Mutex<u32> = Mutex::new(0);
    //~^ mutex_integer

    let mtx = Mutex::new(0);
    //~^ mutex_integer
    // This will still lint, since we're reassigning the mutex to a variable -- oh well.
    // But realistically something like this won't really come up.
    let reassigned = mtx;
    //~^ mutex_integer

    // don't eat the `)` when removing the type ascription -- see
    // https://github.com/rust-lang/rust-clippy/issues/15377
    let (funky_mtx): Mutex<u64> = Mutex::new(0);
    //~^ mutex_integer
}
