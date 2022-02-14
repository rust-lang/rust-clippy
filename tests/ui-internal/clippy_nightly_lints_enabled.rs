// rustc-env:CLIPPY_NIGHTLY=1

// This should trigger `clippy::forever_nightly_lint`
// The lint is warn by default

fn trigger_forever_nightly_lint() {}

fn main() {}
