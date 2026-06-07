#![warn(clippy::large_futures)]
//@no-rustfix

fn main() {}

//~v large_futures
pub async fn should_warn() {
    let x = [0u8; 1024];
    async {}.await;
    dbg!(x);
}
