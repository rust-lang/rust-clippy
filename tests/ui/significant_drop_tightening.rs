//@run-rustfix

#![warn(clippy::significant_drop_tightening)]

use std::{
    cell::UnsafeCell,
    mem::{drop as unlock, MaybeUninit},
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        Mutex,
    },
    task::Waker,
};

pub fn complex_return_triggers_the_lint() -> i32 {
    fn foo() -> i32 {
        1
    }
    let mutex = Mutex::new(1);
    let lock = mutex.lock().unwrap();
    let _ = *lock;
    let _ = *lock;
    foo()
}

pub fn issue_10413() {
    let mutex = Mutex::new(Some(1));
    let opt = Some(1);
    if opt.is_some() {
        let lock = mutex.lock().unwrap();
        let _ = *lock;
        if opt.is_some() {
            let _ = *lock;
        }
    }
}

pub fn issue_11125() {
    pub struct Channel<T> {
        pub inner: Inner,
        pub slots: [UnsafeCell<MaybeUninit<T>>],
    }

    pub struct Inner {
        pub join_wakers: Mutex<Vec<Waker>>,
        pub ref_count: AtomicUsize,
        pub sender_wakers: Mutex<Vec<Waker>>,
        pub status: AtomicU64,
    }

    pub struct SendValue<'s, T> {
        pub channel: &'s Channel<T>,
        pub registered_waker: Option<Waker>,
        pub value: Option<T>,
    }

    impl<'s, T> Drop for SendValue<'s, T> {
        fn drop(&mut self) {
            if let Some(waker) = self.registered_waker.take() {
                let mut sender_wakers = self.channel.inner.sender_wakers.lock().unwrap();
                let idx_opt = sender_wakers.iter().position(|w| w.will_wake(&waker));
                if let Some(idx) = idx_opt {
                    let local_waker = sender_wakers.swap_remove(idx);
                    unlock(sender_wakers);
                    drop(local_waker);
                }
            }
        }
    }
}

pub fn issue_11128() {
    struct Foo {
        droppable: Option<Vec<i32>>,
        mutex: Mutex<Vec<i32>>,
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            if let Some(droppable) = self.droppable.take() {
                let lock = self.mutex.lock().unwrap();
                let idx_opt = lock.iter().copied().find(|el| Some(el) == droppable.first());
                if let Some(idx) = idx_opt {
                    let local_droppable = vec![lock.first().copied().unwrap_or_default()];
                    unlock(lock);
                    drop(local_droppable);
                }
            }
        }
    }
}

pub fn path_return_can_be_ignored() -> i32 {
    let mutex = Mutex::new(1);
    let lock = mutex.lock().unwrap();
    let rslt = *lock;
    let _ = *lock;
    rslt
}

pub fn post_bindings_can_be_ignored() {
    let mutex = Mutex::new(1);
    let lock = mutex.lock().unwrap();
    let rslt = *lock;
    let another = rslt;
    let _ = another;
}

pub fn unnecessary_contention_with_multiple_owned_results() {
    {
        let mutex = Mutex::new(1i32);
        let lock = mutex.lock().unwrap();
        let _ = lock.abs();
        let _ = lock.is_positive();
    }

    {
        let mutex = Mutex::new(1i32);
        let lock = mutex.lock().unwrap();
        let rslt0 = lock.abs();
        let rslt1 = lock.is_positive();
        do_heavy_computation_that_takes_time((rslt0, rslt1));
    }
}

pub fn unnecessary_contention_with_single_owned_results() {
    {
        let mutex = Mutex::new(1i32);
        let lock = mutex.lock().unwrap();
        let _ = lock.abs();
    }
    {
        let mutex = Mutex::new(vec![1i32]);
        let mut lock = mutex.lock().unwrap();
        lock.clear();
    }

    {
        let mutex = Mutex::new(1i32);
        let lock = mutex.lock().unwrap();
        let rslt0 = lock.abs();
        do_heavy_computation_that_takes_time(rslt0);
    }
    {
        let mutex = Mutex::new(vec![1i32]);
        let mut lock = mutex.lock().unwrap();
        lock.clear();
        do_heavy_computation_that_takes_time(());
    }
}

// Marker used for illustration purposes.
pub fn do_heavy_computation_that_takes_time<T>(_: T) {}

fn main() {}
