#![warn(clippy::large_futures)]

fn main() {}

// Note: large_futures are allowed here, as rustfix cannot actually fix this case.
// The reason we still keep it around is that it's used as a helper in other tests.
// See large_futures_unfixable.rs where this definition is demonstrated to emit a lint as well.
#[allow(clippy::large_futures)]
pub async fn should_warn() {
    let x = [0u8; 1024];
    async {}.await;
    dbg!(x);
}

pub async fn should_not_warn() {
    // Note: not 1020 bytes as expected, because after being corrected to `Box::pin(should_warn())`,
    // `bar()` is now the largest future in the tree, and with this being 1020 bytes `bar()` is larger
    // than 1024 bytes because of the added discriminant overhead.
    let x = [0u8; 1012];
    async {}.await;
    dbg!(x);
}

pub async fn bar() {
    should_warn().await;
    //~^ large_futures

    async {
        let x = [0u8; 1024];
        dbg!(x);
    }
    .await;

    should_not_warn().await;
}
