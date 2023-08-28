#![warn(clippy::missing_spin_loop)]
#![allow(clippy::bool_comparison)]
#![allow(unused_braces)]

use core::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let b = AtomicBool::new(true);
    // Those should lint
    while b.load(Ordering::Acquire) {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint
    //~| NOTE: `-D clippy::missing-spin-loop` implied by `-D warnings`

    while !b.load(Ordering::SeqCst) {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint

    while b.load(Ordering::Acquire) == false {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint

    while { true == b.load(Ordering::Acquire) } {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint

    while b.compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed) != Ok(true) {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint

    while Ok(false) != b.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {}
    //~^ ERROR: busy-waiting loop should at least have a spin loop hint

    // This is OK, as the body is not empty
    while b.load(Ordering::Acquire) {
        std::hint::spin_loop()
    }
    // TODO: also match on loop+match or while let
}
