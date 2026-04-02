#![deny(clippy::bind_instead_of_map)]
#![allow(unused_must_use)]
#![allow(
    deprecated,
    reason = "`fetch_update` will be a deprecated alias to `try_update` starting in 1.99,
    but we still want to lint both"
)]

use std::sync::atomic::*;

#[clippy::msrv = "1.94"]
fn msrv_1_94() {
    let x = AtomicBool::new(true);
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, |old| Some(!old));
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, Some);
    let x = AtomicUsize::new(0);
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, |old| {
        if old == 0 { Some(0) } else { Some(old - 1) }
    });
}

#[clippy::msrv = "1.95"]
fn msrv_1_95() {
    let x = AtomicBool::new(true);
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, |old| Some(!old));
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, Some);
    x.try_update(Ordering::Relaxed, Ordering::SeqCst, |old| Some(!old));
    x.try_update(Ordering::Relaxed, Ordering::SeqCst, Some);
    let x = AtomicUsize::new(0);
    x.fetch_update(Ordering::Relaxed, Ordering::SeqCst, |old| {
        if old == 0 { Some(0) } else { Some(old - 1) }
    });
    x.try_update(Ordering::Relaxed, Ordering::SeqCst, |old| {
        if old == 0 { Some(0) } else { Some(old - 1) }
    });
}

fn main() {
    msrv_1_94();
    msrv_1_95();
}
