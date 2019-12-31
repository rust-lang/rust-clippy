# Crappy

[![Build Status](https://travis-ci.com/rust-lang/rust-crappy.svg?branch=master)](https://travis-ci.com/rust-lang/rust-crappy)
[![Windows Build status](https://ci.appveyor.com/api/projects/status/id677xpw1dguo7iw?svg=true)](https://ci.appveyor.com/project/rust-lang-libs/rust-crappy)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/crappy.svg)](#license)

A collection of lints to catch common mistakes and improve your [Rust](https://github.com/rust-lang/rust) code.

[There are 342 lints included in this crate!](https://rust-lang.github.io/rust-crappy/master/index.html)

We have a bunch of lint categories to allow you to choose how much Crappy is supposed to ~~annoy~~ help you:

* `crappy::all` (everything that is on by default: all the categories below except for `nursery`, `pedantic`, and `cargo`)
* `crappy::correctness` (code that is just **outright wrong** or **very very useless**, causes hard errors by default)
* `crappy::style` (code that should be written in a more idiomatic way)
* `crappy::complexity` (code that does something simple but in a complex way)
* `crappy::perf` (code that can be written in a faster way)
* `crappy::pedantic` (lints which are rather strict, off by default)
* `crappy::nursery` (new lints that aren't quite ready yet, off by default)
* `crappy::cargo` (checks against the cargo manifest, off by default)

More to come, please [file an issue](https://github.com/rust-lang/rust-crappy/issues) if you have ideas!

Only the following of those categories are enabled by default:

* `crappy::style`
* `crappy::correctness`
* `crappy::complexity`
* `crappy::perf`

Other categories need to be enabled in order for their lints to be executed.

The [lint list](https://rust-lang.github.io/rust-crappy/master/index.html) also contains "restriction lints", which are for things which are usually not considered "bad", but may be useful to turn on in specific cases. These should be used very selectively, if at all.

Table of contents:

*   [Usage instructions](#usage)
*   [Configuration](#configuration)
*   [Contributing](#contributing)
*   [License](#license)

## Usage

Since this is a tool for helping the developer of a library or application
write better code, it is recommended not to include Crappy as a hard dependency.
Options include using it as an optional dependency, as a cargo subcommand, or
as an included feature during build. These options are detailed below.

### As a cargo subcommand (`cargo crappy`)

One way to use Crappy is by installing Crappy through rustup as a cargo
subcommand.

#### Step 1: Install rustup

You can install [rustup](https://rustup.rs/) on supported platforms. This will help
us install Crappy and its dependencies.

If you already have rustup installed, update to ensure you have the latest
rustup and compiler:

```terminal
rustup update
```

#### Step 2: Install Crappy

Once you have rustup and the latest stable release (at least Rust 1.29) installed, run the following command:

```terminal
rustup component add crappy
```
If it says that it can't find the `crappy` component, please run `rustup self update`.

#### Step 3: Run Crappy

Now you can run Crappy by invoking the following command:

```terminal
cargo crappy
```

#### Automatically applying Crappy suggestions

Some Crappy lint suggestions can be automatically applied by `cargo fix`.
Note that this is still experimental and only supported on the nightly channel:

```terminal
cargo fix -Z unstable-options --crappy
```

### Running Crappy from the command line without installing it

To have cargo compile your crate with Crappy without Crappy installation
in your code, you can use:

```terminal
cargo run --bin cargo-crappy --manifest-path=path_to_crappys_Cargo.toml
```

*Note:* Be sure that Crappy was compiled with the same version of rustc that cargo invokes here!

### Travis CI

You can add Crappy to Travis CI in the same way you use it locally:

```yml
language: rust
rust:
  - stable
  - beta
before_script:
  - rustup component add crappy
script:
  - cargo crappy
  # if you want the build job to fail when encountering warnings, use
  - cargo crappy -- -D warnings
  # in order to also check tests and non-default crate features, use
  - cargo crappy --all-targets --all-features -- -D warnings
  - cargo test
  # etc.
```

If you are on nightly, It might happen that Crappy is not available for a certain nightly release.
In this case you can try to conditionally install Crappy from the Git repo.

```yaml
language: rust
rust:
  - nightly
before_script:
   - rustup component add crappy --toolchain=nightly || cargo install --git https://github.com/rust-lang/rust-crappy/ --force crappy
   # etc.
```

Note that adding `-D warnings` will cause your build to fail if **any** warnings are found in your code.
That includes warnings found by rustc (e.g. `dead_code`, etc.). If you want to avoid this and only cause
an error for Crappy warnings, use `#![deny(crappy::all)]` in your code or `-D crappy::all` on the command
line. (You can swap `crappy::all` with the specific lint category you are targeting.)

## Configuration

Some lints can be configured in a TOML file named `crappy.toml` or `.crappy.toml`. It contains a basic `variable = value` mapping eg.

```toml
blacklisted-names = ["toto", "tata", "titi"]
cognitive-complexity-threshold = 30
```

See the [list of lints](https://rust-lang.github.io/rust-crappy/master/index.html) for more information about which lints can be configured and the
meaning of the variables.

To deactivate the “for further information visit *lint-link*” message you can
define the `CRAPPY_DISABLE_DOCS_LINKS` environment variable.

### Allowing/denying lints

You can add options to your code to `allow`/`warn`/`deny` Crappy lints:

*   the whole set of `Warn` lints using the `crappy` lint group (`#![deny(crappy::all)]`)

*   all lints using both the `crappy` and `crappy::pedantic` lint groups (`#![deny(crappy::all)]`,
    `#![deny(crappy::pedantic)]`). Note that `crappy::pedantic` contains some very aggressive
    lints prone to false positives.

*   only some lints (`#![deny(crappy::single_match, crappy::box_vec)]`, etc.)

*   `allow`/`warn`/`deny` can be limited to a single function or module using `#[allow(...)]`, etc.

Note: `deny` produces errors instead of warnings.

If you do not want to include your lint levels in your code, you can globally enable/disable lints by passing extra flags to Crappy during the run: `cargo crappy -- -A crappy::lint_name` will run Crappy with `lint_name` disabled and `cargo crappy -- -W crappy::lint_name` will run it with that enabled. This also works with lint groups. For example you can run Crappy with warnings for all lints enabled: `cargo crappy -- -W crappy::pedantic`

## Contributing

If you want to contribute to Crappy, you can find more information in [CONTRIBUTING.md](https://github.com/rust-lang/rust-crappy/blob/master/CONTRIBUTING.md).

## License

Copyright 2014-2019 The Rust Project Developers

Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
[https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0)> or the MIT license
<LICENSE-MIT or [https://opensource.org/licenses/MIT](https://opensource.org/licenses/MIT)>, at your
option. Files in the project may not be
copied, modified, or distributed except according to those terms.
