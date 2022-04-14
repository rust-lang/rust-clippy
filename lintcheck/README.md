## `cargo lintcheck`

Runs clippy on a fixed set of crates read from
[`lintcheck/lintcheck_crates/Cargo.toml`] and saves logs of the lint warnings
into the repo.  We can then check the diff and spot new or disappearing
warnings.

From the repo root, run:

```
cargo run --target-dir lintcheck/target --manifest-path lintcheck/Cargo.toml
```

or

```
cargo lintcheck
```

By default the logs will be saved into
`lintcheck-logs/lintcheck_crates_logs.txt`.

You can set a custom sources Cargo.toml by adding
`--crates-toml custom/Cargo.toml` or using `LINTCHECK_TOML="custom/Cargo.toml"`.

The results will then be saved to `lintcheck-logs/custom_logs.toml`.

### Configuring the Crate Sources

The sources to check are saved in a `Cargo.toml` file. Currently only crates.io
dependencies are linted. Packages can be made optional to support `--package`
selection, otherwise they will always be checked.

A list of crates that clippy should not run on can be provided through
`package.metadata.lintcheck.ignore`. See 
[`lintcheck/lintcheck_crates/Cargo.toml`] for an example.

[`lintcheck/lintcheck_crates/Cargo.toml`]: ./lintcheck_crates/Cargo.toml
