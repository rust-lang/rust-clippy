//@no-rustfix
#![warn(clippy::unnecessary_blocking_ops)]
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

mod async_mod {
    pub async fn sleep(_dur: std::time::Duration) {}
    pub async fn read_to_string(mut reader: std::io::Stdin) -> Result<String, ()> {
        Ok(String::new())
    }
}

mod blocking_mod {
    pub async fn sleep(_dur: std::time::Duration) {}
}

pub async fn async_fn() {
    sleep(Duration::from_secs(1));
    //~^ ERROR: blocking function call detected in an async body
    fs::remove_dir("").unwrap();
    //~^ ERROR: blocking function call detected in an async body
    fs::copy("", "_").unwrap();
    //~^ ERROR: blocking function call detected in an async body
    let mut r: &[u8] = b"hello";
    let mut w: Vec<u8> = vec![];
    io::copy(&mut r, &mut w).unwrap();
    //~^ ERROR: blocking function call detected in an async body
    let _cont = io::read_to_string(io::stdin()).unwrap();
    //~^ ERROR: blocking function call detected in an async body
    fs::create_dir("").unwrap();
    //~^ ERROR: blocking function call detected in an async body
}

fn main() {}
