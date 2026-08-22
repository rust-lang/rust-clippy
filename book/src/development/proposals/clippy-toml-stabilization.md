# Clippy.toml stabilization

- Start Date: 2025-09-07

**Progress**
- [ ] Deprecating `CLIPPY_CONF_DIR` and creating `CLIPPY_CONF_PATH` with an improved algorithm
- [ ] Create the `unstable-conf` key and allow for unknown values.
- \[0/88\] Options stabilized

# Summary

Stabilization of the Clippy configuration file and other ways to change Clippy’s behaviour via values passed to the
program at runtime.

This RFC proposes a change to the algorithm that `CLIPPY_CONF_DIR` uses, a deprecation of `CLIPPY_CONF_DIR` in lieu of
`CLIPPY_CONF_PATH` (allowing for custom-named Clippy configuration files), allowing for several configuration files
when on a workspace.

# Motivation

High-stakes projects need a guarantee of stabilization in every aspect of their toolchain. Including configuration
options and default values. One of these high-stakes projects is Rust-for-Linux, which has communicated with the
Clippy team before about stabilizing a way to pass configuration values to Clippy.

It’s our duty as maintainers to make sure that user’s workflows don’t break. And that they don’t worry about future
problems in their pipelines because we haven’t formerly stabilized a configuration file.

[Kernel thread talking about how clippy.toml being unstable is worrisome](https://lore.kernel.org/all/20250310170458.594728659@linuxfoundation.org/)
[Zulip thread](https://rust-lang.zulipchat.com/#narrow/channel/257328-clippy/topic/stablization.20of.20clippy.2Etoml.20a/with/486169617)

# Guide-level explanation

Some Clippy lints can be tweaked via a configuration file commonly named `clippy.toml` or `.clippy.toml`.

Clippy looks for this file in the current directory and all parents up to the root of the workspace if there is one.

For crates in a workspace, the Clippy configuration files would be merged with the ones in their parent directories.
A Clippy configuration file will always overwrite one in a parent directory

This means, that if your workspace has `clippy.toml` and `crates/my_crate/clippy.toml`, when Clippy analyzes
`my_crate`, it will merge both versions with `crates/my_crate/clippy.toml` taking prevalence over `clippy.toml` for
`my_crate`.

This system can be overrun via the `CLIPPY_CONF_PATH` environment variable passed to the `clippy-driver` or
`cargo clippy` invocation. If this environment variable is declared and points to a directory, Clippy just looks up
specifically that directory and assumes that there isn’t a Clippy configuration file if it can’t find `clippy.toml`
or `.clippy.toml`. If `CLIPPY_CONF_PATH` points to a file, that file will be interpreted as `clippy.toml`.

---

The Clippy configuration file follows a subset of the TOML specification. You can declare key-value pairs within it
in plain text. Here's an example.

```toml
unstable-conf = false

msrv = "1.76.0"
avoid-breaking-exported-api = true
check-private-items = true
```

The list of available configurations along their default values is
[here](https://doc.rust-lang.org/nightly/clippy/lint_configuration.html).
You can enable unstable features via the `unstable-conf` configuration option. If this configuration is set to `true`,
Clippy will warn about unknown or renamed keys, and will allow the modification of unstable configuration options. One
can use `unstable-conf = true` sparingly in their development cycle to check for typos or
nonexistent configuration options.

While TOML tables (`[table]`) are currently ignored at the time of writing, they could be added in a future version
for lint-specific configuration or similar.

# Reference-level explanation

Before Clippy initializes, it checks for `CLIPPY_CONF_PATH` environmental variable. If that exists, it checks if its
a directory (in which it performs the known algorithm, looking for all parents up to `/`, merging them...). If
`CLIPPY_CONF_PATH` is a file, just read it as we'd do with `clippy.toml`.

`CLIPPY_CONF_PATH` takes prevalence over `CLIPPY_CONF_DIR`, but if the former doesn't exist, look for the latter. If
none of these exist, look for the usual `clippy.toml`/`.clippy.toml` in the crate's directory.

With merging, we are referring about the configuration keys with more prevalence, completely replacing the less
prevalent ones. Configuration keys which are not mentioned in the most prevalent one would be taken from less
prevalent ones. As a practical example,

```toml
# clippy.toml
msrv = "1.82.0"
disallowed-macros = [ { path = "my_crate::bad_macro" } ]

# crates/my_crate/clippy.toml
msrv = "1.76.0" # This overrides that 1.82.0 for `my_crate`
# We adopt that disallowed-macros from the outer clippy.toml
```

## Stabilization process for keys

Each key needs to be independently stabilized, or it won't be active on configurations without `unstable-conf` enabled.
This is done via opening a Github PR marking it as no longer unstable, having fixed all issues related to that key
beforehand.

Things that need to be accounted for before stabilizing a key:

- Are there any lints having problems with this configuration key?
- Is this key mentioned in any issue? Is it a relevant factor to that issue?
- Does this key impact unstable features of the compiler, does it rely on behaviour that only happens on nightly?
- If the key affects macro behaviour, is it well handled?

If necessary, perform some tests on well-known and stable codebases before merging, either via lintcheck, crater or
manually.

One can use @rust-rfcbot's capabilities to facilitate registering concerns and closing them with
`@rust-rfcbot concern <concern name>`. When all concerns are fixed, a team member will use the `@rust-rfcbot fcp merge`
command. [Refer to rust-rfcbot's documentation for more information](https://github.com/rust-lang/rfcbot-rs)
but this should be enough for our usecase.

While a key is not stable, it's marked on documentation as unstable, and it will report an error if used without
`unstable-conf` enabled.

---

When `unstable-conf` is not enabled, we are in "stable mode". This means that unknown configuration lints are
suppressed, renamed configuration lints are supressed, and we only allow the use of stable configuration options,
emitting an error otherwise.

# Drawbacks

- There isn't a good way to specify configuration values from the terminal, for the time being.
- In stable mode, typos and/or deprecated and/or renamed configuration keys being renamed cannot be warned against.

# Prior art

- `rustfmt.toml`
- `.cargo/config.toml`
- `rust-toolchain.toml`
