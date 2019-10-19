#![allow(clippy::all)]
#![warn(clippy::mutex_mutable_self)]
#![allow(unused)]

use std::sync::Mutex;


fn get_value_mut(m: &mut Mutex<u32>) -> u32 {
    *m.lock().unwrap()
}

fn get_value(m: &Mutex<u32>) -> u32 {
    *m.lock().unwrap()
}

fn mk_mutex() -> Mutex<u32> {
    Mutex::new(10)
}

fn main() {
    let mut m = Mutex::new(42);
    *m.lock().unwrap() = 64;

    let _ = get_value(&m);
    let _ = get_value_mut(&mut m);

    let _ = mk_mutex().lock().unwrap();
}
