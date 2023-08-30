#![warn(clippy::explicit_write)]
#![allow(unused_imports)]
#![allow(clippy::uninlined_format_args)]

fn stdout() -> String {
    String::new()
}

fn stderr() -> String {
    String::new()
}

macro_rules! one {
    () => {
        1
    };
}

fn main() {
    // these should warn
    {
        use std::io::Write;
        write!(std::io::stdout(), "test").unwrap();
        //~^ ERROR: use of `write!(stdout(), ...).unwrap()`
        //~| NOTE: `-D clippy::explicit-write` implied by `-D warnings`
        write!(std::io::stderr(), "test").unwrap();
        //~^ ERROR: use of `write!(stderr(), ...).unwrap()`
        writeln!(std::io::stdout(), "test").unwrap();
        //~^ ERROR: use of `writeln!(stdout(), ...).unwrap()`
        writeln!(std::io::stderr(), "test").unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
        std::io::stdout().write_fmt(format_args!("test")).unwrap();
        //~^ ERROR: use of `stdout().write_fmt(...).unwrap()`
        std::io::stderr().write_fmt(format_args!("test")).unwrap();
        //~^ ERROR: use of `stderr().write_fmt(...).unwrap()`

        // including newlines
        writeln!(std::io::stdout(), "test\ntest").unwrap();
        //~^ ERROR: use of `writeln!(stdout(), ...).unwrap()`
        writeln!(std::io::stderr(), "test\ntest").unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`

        let value = 1;
        writeln!(std::io::stderr(), "with {}", value).unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
        writeln!(std::io::stderr(), "with {} {}", 2, value).unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
        writeln!(std::io::stderr(), "with {value}").unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
        writeln!(std::io::stderr(), "macro arg {}", one!()).unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
        let width = 2;
        writeln!(std::io::stderr(), "{:w$}", value, w = width).unwrap();
        //~^ ERROR: use of `writeln!(stderr(), ...).unwrap()`
    }
    // these should not warn, different destination
    {
        use std::fmt::Write;
        let mut s = String::new();
        write!(s, "test").unwrap();
        write!(s, "test").unwrap();
        writeln!(s, "test").unwrap();
        writeln!(s, "test").unwrap();
        s.write_fmt(format_args!("test")).unwrap();
        s.write_fmt(format_args!("test")).unwrap();
        write!(stdout(), "test").unwrap();
        write!(stderr(), "test").unwrap();
        writeln!(stdout(), "test").unwrap();
        writeln!(stderr(), "test").unwrap();
        stdout().write_fmt(format_args!("test")).unwrap();
        stderr().write_fmt(format_args!("test")).unwrap();
    }
    // these should not warn, no unwrap
    {
        use std::io::Write;
        std::io::stdout().write_fmt(format_args!("test")).expect("no stdout");
        std::io::stderr().write_fmt(format_args!("test")).expect("no stderr");
    }
}
