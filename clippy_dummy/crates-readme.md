This is just a dummy crate to reserve the name clippy_utils. We currently [don't
plan](https://github.com/rust-lang/rust-clippy/pull/6746#issuecomment-780747522) to publish this crate on crates.io, but
we want to keep the option of doing so in the future.

To use this, add `clippy_utils` as a git dependency:

```toml
clippy_utils = { git = "https://github.com/rust-lang/rust-clippy", rev = "<sha>" }
```
