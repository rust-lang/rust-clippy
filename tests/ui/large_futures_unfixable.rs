#![warn(clippy::large_futures)]
//@no-rustfix

async fn big_fut(_arg: [u8; 1024 * 16]) {}
//~^ large_futures
