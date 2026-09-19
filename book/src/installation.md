# Installation

If you're using `rustup` to install and manage your Rust toolchains, Clippy is
usually **already installed**. In that case you can skip this chapter and go to
the [Usage] chapter.

> Note: If you used the `minimal` profile when installing a Rust toolchain,
> Clippy is not automatically installed.

## Using Rustup

If Clippy was not installed for a toolchain, it can be installed with

```
$ rustup component add clippy [--toolchain=<name>]
```

### Use a specific version of Clippy

Clippy may introduce new warnings and errors between associated Rust versions.
This may be desirable if you want to learn about newly detectable issues in your
codebase over time, but it can cause [continuous integration](./continuous_integration/index.md)
to fail even if you haven't touched your Rust code. If you'd like to keep Clippy's
behaviour stable for your project, use
[`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
to pin your entire Rust toolchain. For example:

```toml
[toolchain]
channel = "1.83.0"
components = ["clippy"]
```

## From Source

Take a look at the [Basics] chapter in the Clippy developer guide to find step-by-step
instructions on how to build and install Clippy from source.

[Basics]: development/basics.md#install-from-source
[Usage]: usage.md
