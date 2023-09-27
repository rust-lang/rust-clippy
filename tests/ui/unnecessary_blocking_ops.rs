//@no-rustfix
#![feature(async_fn_in_trait)]
#![feature(async_closure)]
#![allow(incomplete_features)]
#![warn(clippy::unnecessary_blocking_ops)]
use std::{fs, io};
use std::thread::sleep;
use std::time::Duration;
use tokio::io as tokio_io;

mod totally_thread_safe {
    pub async fn sleep(_dur: std::time::Duration) {}
}

pub async fn async_fn() {
    sleep(Duration::from_secs(1));
    fs::remove_dir("").unwrap();
    fs::copy("", "_").unwrap();
    let _ = fs::canonicalize("");

    {
        fs::write("", "").unwrap();
        let _ = io::stdin();
    }
    let _stdout = io::stdout();
    let mut r: &[u8] = b"hello";
    let mut w: Vec<u8> = vec![];
    io::copy(&mut r, &mut w).unwrap();
}

pub async fn non_blocking() {
    totally_thread_safe::sleep(Duration::from_secs(1)).await; // don't lint, not blocking
    
    
    let mut r: &[u8] = b"hello";
    let mut w: Vec<u8> = vec![];
    tokio_io::copy(&mut r, &mut w).await; // don't lint, not blocking
}

trait AsyncTrait {
    async fn foo(&self);
}

struct SomeType(u8);
impl AsyncTrait for SomeType {
    async fn foo(&self) {
        sleep(Duration::from_secs(self.0 as _));
    }
}

fn do_something() {}

fn closures() {
    let _ = async || sleep(Duration::from_secs(1));
    let async_closure = async move |_a: i32| {
        let _ = 1;
        do_something();
        sleep(Duration::from_secs(1));
    };
    let non_async_closure = |_a: i32| {
        sleep(Duration::from_secs(1)); // don't lint, not async
        do_something();
    };
}

fn main() {}
