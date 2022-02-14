#![warn(clippy::await_holding_span_guard)]

use tracing::{span, Instrument, Level, Span};

async fn good_in_scope() {
    let span = span!(Level::INFO, "good");

    let some_value = span.in_scope(|| 32);

    baz(some_value).await;
}

async fn good_instrument() {
    let span = span!(Level::INFO, "good");

    baz(32).instrument(span).await;
}

async fn bad_borrowed() {
    let span = span!(Level::INFO, "bad_borrowed");

    let _guard = span.enter();
    bar().await;
}

async fn bad_owned() {
    let span = span!(Level::INFO, "bad_owned");

    let _guard = span.entered();
    bar().await;
}

#[allow(clippy::manual_async_fn)]
fn bad_async_block_borrowed() -> impl std::future::Future<Output = ()> + 'static {
    async move {
        let span = span!(Level::INFO, "async_block_borrowed");
        let _guard = span.enter();
        bar().await
    }
}

async fn bar() {}

async fn baz(value: usize) {
    let _ = value;
}

fn main() {
    good_in_scope();
    good_instrument();
    bad_borrowed();
    bad_owned();
    bad_async_block_borrowed();
}
