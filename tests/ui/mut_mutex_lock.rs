#![warn(clippy::mut_mutex_lock)]

use std::sync::{Arc, Mutex};

fn mut_mutex_lock() {
    let mut value_rc = Arc::new(Mutex::new(42_u8));
    let value_mutex = Arc::get_mut(&mut value_rc).unwrap();

    let mut value = value_mutex.lock().unwrap();
    //~^ mut_mutex_lock
    *value += 1;

    let mut value_mutex = Mutex::new(42_u8);
    let mut_ref_mut_ref_mutex = &mut &mut value_mutex;
    let mut value = mut_ref_mut_ref_mutex.lock().unwrap();
    //~^ mut_mutex_lock
    *value += 1;
}

fn no_owned_mutex_lock() {
    let mut value_rc = Arc::new(Mutex::new(42_u8));
    let mut value = value_rc.lock().unwrap();
    *value += 1;
}

fn issue9415() {
    let mut arc_mutex = Arc::new(Mutex::new(42_u8));
    let arc_mutex: &mut Arc<Mutex<u8>> = &mut arc_mutex;
    let mut guard = arc_mutex.lock().unwrap();
    *guard += 1;
}

fn mut_ref_ref_mutex_lock() {
    let mutex = Mutex::new(42_u8);
    let mut_ref_ref_mutex = &mut &mutex;
    let mut guard = mut_ref_ref_mutex.lock().unwrap();
    *guard += 1;
}

mod issue16253 {
    use std::sync::{Arc, Mutex};

    // Do not lint, even if the mutex is owned
    fn dont_lint_owned(m: Mutex<i32>) {
        m.lock();
    }

    struct Wrapper {
        arc: Arc<Mutex<i32>>,
        ref_: &'static Mutex<i32>,
        ref_mut: &'static mut Mutex<i32>,
        owned: Mutex<i32>,
    }

    // Do not lint, even if the projection chain would theoretically allow mutable access
    fn field(w: Wrapper, ref_w: &Wrapper, refmut_w: &mut Wrapper) {
        w.arc.lock();
        w.ref_.lock();
        w.ref_mut.lock();
        w.owned.lock();
        ref_w.arc.lock();
        ref_w.ref_.lock();
        ref_w.ref_mut.lock();
        ref_w.owned.lock();
        refmut_w.arc.lock();
        refmut_w.ref_.lock();
        refmut_w.ref_mut.lock();
        refmut_w.owned.lock();
    }

    // Do not lint, even if the `.index()` could've been turned into `.index_mut()`
    fn index(mutexes: &mut [Mutex<u32>]) {
        // even though `[0]` is _currently_ an `.index(0)`, it can be turned into `.index_mut()` to
        // enable mutable access: `&mut [Mutex] -> &mut Mutex`
        mutexes[0].lock().unwrap();

        // `exes` is `&[Mutex] = &Mutex`, so we can't get to `&mut Mutex` no matter what
        let exes: &_ = mutexes;
        exes[0].lock().unwrap();
    }
}

fn main() {}
