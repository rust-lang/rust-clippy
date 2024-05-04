# Syncing changes between Clippy and [`rust-lang/rust`]

Clippy currently gets built with a pinned nightly version.

In the `rust-lang/rust` repository, where rustc resides, there's a copy of
Clippy that compiler devs modify from time to time to adapt to changes in the
unstable API of the compiler.

We need to sync these changes back to this repository periodically, and the
changes made to this repository in the meantime also need to be synced to the
`rust-lang/rust` repository.

To avoid flooding the `rust-lang/rust` PR queue, this two-way sync process is
done in a bi-weekly basis if there's no urgent changes. This is done starting on
the day of the Rust stable release and then every other week. That way we
guarantee that we keep this repo up to date with the latest compiler API, and
every feature in Clippy is available for 2 weeks in nightly, before it can get
to beta. For reference, the first sync following this cadence was performed on
2020-08-27.

This process is described in detail in the following sections.

## Installing `josh-proxy`

The sync is done with [JOSH] and fully scripted with `cargo dev sync`. The only
requirement is to install the `josh-proxy` binary from GitHub

<!-- FIXME: Change to a release version once >r23.12.04 is released -->

```sh
$ RUSTFLAGS="--cap-lints warn" cargo +stable install josh-proxy --git https://github.com/josh-project/josh
```

[JOSH]: https://josh-project.github.io/josh/

## Performing the sync from [`rust-lang/rust`] to Clippy

Doing the sync now is just running

```
$ cargo dev sync pull
```

This command will update the nightly toolchain in the `rust-toolchain` file and
will pull the changes from the Rust repository.

If there should be merge conflicts, resolve them now and commit with the message
`Merge from rustc`.[^1]

> Note: If the version tests fail, refer to [bump version] in the release
> documentation.

Open a PR to `rust-lang/rust-clippy` and if you are a Clippy maintainer, you can
`r+` the PR yourself. If not, change `r? @ghost` to `r? clippy` and a Clippy
maintainer will get assigned. To accelerate the process ping the Clippy team on
[Zulip].


[bump version]: release.md#bump-version
[Zulip]: https://rust-lang.zulipchat.com/#narrow/stream/clippy

[^1]: The message is not really important, but consistency is nice.

## Performing the sync from Clippy to [`rust-lang/rust`]

The other direction is done by running

```
$ cargo dev sync push /path/to/rust --user <GitHub-name>
```

Where the `/path/to/rust` is a relative path to a Rust clone and the
`<GitHub-name>` is your GitHub user name. This is required for pushing the sync
to GitHub and opening a PR.

If everything went right, there will be a GitHub link that has to be used to
open the sync PR in the Rust repository. The PR description must look like this:

```
Clippy subtree update

r? @ghost

Sync from Clippy commit: rust-lang/rust-clippy@<sha1>
```

The title must be kept as is, to [tell triagebot] that this is a sync PR.

The second line must be kept as is, to [find the Clippy commit] during a
release.

[find the Clippy commit]: release.md#find-the-clippy-commit
[tell triagebot]: https://github.com/rust-lang/rust/pull/114157

[`rust-lang/rust`]: https://github.com/rust-lang/rust
