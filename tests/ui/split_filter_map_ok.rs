#![allow(clippy::map_identity)]
#![warn(clippy::split_filter_map_ok)]

use std::io::{self, BufRead, BufReader};

fn main() -> io::Result<()> {
    // Lint:

    let f = std::fs::File::open("/")?;
    BufReader::new(f).split(0).filter_map(Result::ok).for_each(|_| ());
    //~^ split_filter_map_ok
    let f = std::fs::File::open("/")?;
    BufReader::new(f).split(0).flat_map(Result::ok).for_each(|_| ());
    //~^ split_filter_map_ok
    let f = std::fs::File::open("/")?;
    BufReader::new(f).split(0).flatten().for_each(|_| ());
    //~^ split_filter_map_ok

    io::stdin().lock().split(0).filter_map(Result::ok).for_each(|_| ());
    //~^ split_filter_map_ok
    io::stdin().lock().split(0).filter_map(|x| x.ok()).for_each(|_| ());
    //~^ split_filter_map_ok
    io::stdin().lock().split(0).flatten().for_each(|_| ());
    //~^ split_filter_map_ok

    // Do not lint:

    // not an `std::io::Split` iterator
    io::stdin()
        .lock()
        .split(0)
        .map(std::convert::identity)
        .filter_map(|x| x.ok())
        .for_each(|_| ());
    // not a `Result::ok()` extractor
    io::stdin().lock().split(0).filter_map(|x| x.err()).for_each(|_| ());
    Ok(())
}

#[clippy::msrv = "1.56"]
fn msrv_check() {
    let _lines = BufReader::new(std::fs::File::open("some-path").unwrap())
        .split(0)
        .filter_map(Result::ok);
}
