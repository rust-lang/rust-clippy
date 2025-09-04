#![warn(clippy::discarded_future)]
#![allow(clippy::result_unit_err)]

pub fn result_future() -> Result<impl std::future::Future<Output = ()>, ()> {
    Ok(async {
        dbg!("hello im in a future");
    })
}

pub async fn calls_result_future() {
    // some async stuff
    let _ = result_future();
    //~^ discarded_future
    // more async stuff
}

fn main() {

}