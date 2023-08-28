// Issue #5746
#![warn(clippy::redundant_pattern_matching)]
#![allow(
    clippy::if_same_then_else,
    clippy::equatable_if_let,
    clippy::needless_if,
    clippy::needless_else
)]
use std::task::Poll::{Pending, Ready};

fn main() {
    let m = std::sync::Mutex::new((0, 0));

    // Result
    if let Ok(_) = m.lock() {}
    //~^ ERROR: redundant pattern matching, consider using `is_ok()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    if let Err(_) = Err::<(), _>(m.lock().unwrap().0) {}
    //~^ ERROR: redundant pattern matching, consider using `is_err()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries

    {
        if let Ok(_) = Ok::<_, std::sync::MutexGuard<()>>(()) {}
        //~^ ERROR: redundant pattern matching, consider using `is_ok()`
        //~| NOTE: this will change drop order of the result, as well as all temporaries
    }
    if let Ok(_) = Ok::<_, std::sync::MutexGuard<()>>(()) {
    //~^ ERROR: redundant pattern matching, consider using `is_ok()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    } else {
    }
    if let Ok(_) = Ok::<_, std::sync::MutexGuard<()>>(()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ok()`
    if let Err(_) = Err::<std::sync::MutexGuard<()>, _>(()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_err()`

    if let Ok(_) = Ok::<_, ()>(String::new()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ok()`
    if let Err(_) = Err::<(), _>((String::new(), ())) {}
    //~^ ERROR: redundant pattern matching, consider using `is_err()`

    // Option
    if let Some(_) = Some(m.lock()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    if let Some(_) = Some(m.lock().unwrap().0) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries

    {
        if let None = None::<std::sync::MutexGuard<()>> {}
        //~^ ERROR: redundant pattern matching, consider using `is_none()`
        //~| NOTE: this will change drop order of the result, as well as all temporaries
    }
    if let None = None::<std::sync::MutexGuard<()>> {
    //~^ ERROR: redundant pattern matching, consider using `is_none()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    } else {
    }

    if let None = None::<std::sync::MutexGuard<()>> {}
    //~^ ERROR: redundant pattern matching, consider using `is_none()`

    if let Some(_) = Some(String::new()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`
    if let Some(_) = Some((String::new(), ())) {}
    //~^ ERROR: redundant pattern matching, consider using `is_some()`

    // Poll
    if let Ready(_) = Ready(m.lock()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    if let Ready(_) = Ready(m.lock().unwrap().0) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries

    {
        if let Pending = Pending::<std::sync::MutexGuard<()>> {}
        //~^ ERROR: redundant pattern matching, consider using `is_pending()`
        //~| NOTE: this will change drop order of the result, as well as all temporaries
    }
    if let Pending = Pending::<std::sync::MutexGuard<()>> {
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`
    //~| NOTE: this will change drop order of the result, as well as all temporaries
    } else {
    }

    if let Pending = Pending::<std::sync::MutexGuard<()>> {}
    //~^ ERROR: redundant pattern matching, consider using `is_pending()`

    if let Ready(_) = Ready(String::new()) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
    if let Ready(_) = Ready((String::new(), ())) {}
    //~^ ERROR: redundant pattern matching, consider using `is_ready()`
}
