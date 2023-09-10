#![warn(clippy::zombie_processes)]

use std::process::{Child, Command};

fn main() {
    let _ = Command::new("").spawn().unwrap();
    //~^ ERROR: spawned process is never `wait()`-ed on
    Command::new("").spawn().unwrap();
    //~^ ERROR: spawned process is never `wait()`-ed on
    spawn_proc();
    //~^ ERROR: spawned process is never `wait()`-ed on
    spawn_proc().wait().unwrap(); // OK

    {
        let mut x = Command::new("").spawn().unwrap();
        //~^ ERROR: spawned process is never `wait()`-ed on
        x.kill();
        x.id();
    }
    {
        let mut x = Command::new("").spawn().unwrap();
        x.wait().unwrap(); // OK
    }
    {
        let x = Command::new("").spawn().unwrap();
        x.wait_with_output().unwrap(); // OK
    }
    {
        let mut x = Command::new("").spawn().unwrap();
        x.try_wait().unwrap(); // OK
    }
    {
        let mut x = Command::new("").spawn().unwrap();
        let mut r = &mut x;
        r.wait().unwrap(); // OK, not calling `.wait()` directly on `x` but through `r` -> `x`
    }
    {
        let mut x = Command::new("").spawn().unwrap();
        process_child(x); // OK, other function might call `.wait()` so assume it does
    }
    {
        let mut x = Command::new("").spawn().unwrap();
        //~^ ERROR: spawned process is never `wait()`-ed on
        let v = &x;
        // (allow shared refs is fine because one cannot call `.wait()` through that)
    }
}

fn spawn_proc() -> Child {
    todo!()
}

fn process_child(c: Child) {
    todo!()
}
