//! This test is meant to only be run in CI. To run it locally use:
//!
//! `env INTEGRATION=rust-lang/log cargo test --test integration --features=integration`
//!
//! You can use a different `INTEGRATION` value to test different repositories.
//!
//! This test will clone the specified repository and run Clippy on it. The test succeeds, if
//! Clippy doesn't produce an ICE. Lint warnings are ignored by this test.

#![cfg(feature = "integration")]
#![cfg_attr(feature = "deny-warnings", deny(warnings))]
#![warn(rust_2018_idioms, unused_lifetimes)]

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

#[cfg(not(windows))]
const CARGO_CLIPPY: &str = "cargo-clippy";
#[cfg(windows)]
const CARGO_CLIPPY: &str = "cargo-clippy.exe";

#[cfg_attr(feature = "integration", test)]
fn integration_test() {
    let repo_name = env::var("INTEGRATION").expect("`INTEGRATION` var not set");

    if repo_name == "rust-lang/rust" {
        return;
    }

    let repo_url = format!("https://github.com/{repo_name}");
    let crate_name = repo_name
        .split('/')
        .nth(1)
        .expect("repo name should have format `<org>/<name>`");

    let mut repo_dir = tempfile::tempdir().expect("couldn't create temp dir").into_path();
    repo_dir.push(crate_name);

    let st = Command::new("git")
        .args([
            OsStr::new("clone"),
            OsStr::new("--depth=1"),
            OsStr::new(&repo_url),
            OsStr::new(&repo_dir),
        ])
        .status()
        .expect("unable to run git");
    assert!(st.success());

    let root_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::path::Path::new(&root_dir).join("target");
    let clippy_binary = target_dir.join(env!("PROFILE")).join(CARGO_CLIPPY);

    let output = Command::new(clippy_binary)
        .current_dir(repo_dir)
        .env("RUST_BACKTRACE", "full")
        .env("CARGO_TARGET_DIR", target_dir)
        .args([
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "--cap-lints",
            "warn",
            "-Wclippy::pedantic",
            "-Wclippy::nursery",
        ])
        .output()
        .expect("unable to run clippy");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // debug:
    eprintln!("{stderr}");

    // this is an internal test to make sure we would correctly panic on a delay_span_bug
    if repo_name == "matthiaskrgr/clippy_ci_panic_test" {
        // we need to kind of switch around our logic here:
        // if we find a panic, everything is fine, if we don't panic, SOMETHING is broken about our testing

        // the repo basically just contains a delay_span_bug that forces rustc/clippy to panic:
        /*
           #![feature(rustc_attrs)]
           #[rustc_error(delay_span_bug_from_inside_query)]
           fn main() {}
        */

        if stderr.find("error: internal compiler error").is_some() {
            eprintln!("we saw that we intentionally panicked, yay");
            return;
        }

        panic!("panic caused by delay_span_bug was NOT detected! Something is broken!");
    }

    if let Some(backtrace_start) = stderr.find("error: internal compiler error") {
        static BACKTRACE_END_MSG: &str = "end of query stack";
        let backtrace_end = stderr[backtrace_start..]
            .find(BACKTRACE_END_MSG)
            .expect("end of backtrace not found");

        panic!(
            "internal compiler error\nBacktrace:\n\n{}",
            &stderr[backtrace_start..backtrace_start + backtrace_end + BACKTRACE_END_MSG.len()]
        );
    } else if stderr.contains("query stack during panic") {
        panic!("query stack during panic in the output");
    } else if stderr.contains("E0463") {
        // Encountering E0463 (can't find crate for `x`) did _not_ cause the build to fail in the
        // past. Even though it should have. That's why we explicitly panic here.
        // See PR #3552 and issue #3523 for more background.
        panic!("error: E0463");
    } else if stderr.contains("E0514") {
        panic!("incompatible crate versions");
    } else if stderr.contains("failed to run `rustc` to learn about target-specific information") {
        panic!("couldn't find librustc_driver, consider setting `LD_LIBRARY_PATH`");
    } else {
        assert!(
            !stderr.contains("toolchain") || !stderr.contains("is not installed"),
            "missing required toolchain"
        );
    }

    match output.status.code() {
        Some(0) => println!("Compilation successful"),
        Some(code) => eprintln!("Compilation failed. Exit code: {code}"),
        None => panic!("Process terminated by signal"),
    }
}

#[cfg_attr(feature = "integration", test)]
fn integration_test_rustc() {
    let repo_name = env::var("INTEGRATION").expect("`INTEGRATION` var not set");

    // try to avoid running this test locally
    if !(repo_name == "rust-lang/rust" && env::var("GITHUB_ACTIONS") == Ok(String::from("true"))) {
        return;
    }

    let repo_url = format!("https://github.com/{repo_name}");
    let crate_name = repo_name
        .split('/')
        .nth(1)
        .expect("repo name should have format `<org>/<name>`");

    let mut repo_dir = tempfile::tempdir().expect("couldn't create temp dir").into_path();
    repo_dir.push(crate_name);

    dbg!("cloning git repo");
    let st_git_cl = Command::new("git")
        .args([
            OsStr::new("clone"),
            OsStr::new("--depth=5000"),
            OsStr::new(&repo_url),
            OsStr::new(&repo_dir),
        ])
        .status()
        .expect("unable to run git");
    assert!(st_git_cl.success());

    dbg!("getting rustc version");
    // clippy is pinned to a specific nightly version
    // check out the commit of that nightly to ensure compatibility
    let rustc_output = Command::new("rustc")
        .arg("--version")
        .arg("--verbose")
        .output()
        .expect("failed to run rustc --version");

    let commit_line = String::from_utf8_lossy(&rustc_output.stdout);
    let commit_line_ = commit_line
        .lines()
        .find(|line| line.starts_with("commit-hash: "))
        .expect("did not find 'commit-hash:' in --version output");

    let commit = commit_line_
        .strip_prefix("commit-hash: ")
        .expect("failed parsing commit line");

    dbg!(&commit);
    // check out the commit in the rustc repo to ensure clippy is compatible

    dbg!("checking out commit in rustc repo");
    let st_git_checkout = Command::new("git")
        .current_dir(&repo_dir)
        .arg("checkout")
        .arg(commit)
        .status()
        .expect("git failed to check out commit");
    assert!(st_git_checkout.success());

    let root_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::path::Path::new(&root_dir).join("target");
    let clippy_exec_dir = target_dir.join(env!("PROFILE"));

    // we need to make sure that `x.py clippy` picks up our self-built clippy
    // try to make the target dir discoverable as PATH

    dbg!(&repo_dir);
    assert!(repo_dir.is_dir(), "repo_dir not a dir!");

    let path_env = std::env::var_os("PATH").expect("PATH env var not set");
    let mut paths = env::split_paths(&path_env).collect::<Vec<_>>();
    paths.push(clippy_exec_dir.clone());
    let new_path = env::join_paths(paths).expect("failed to join paths");
    dbg!(&new_path);

    // copy our own clippy binary into the rustc toolchain dir so that x.py finds them
    let sysroot_output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .expect("rustc failed to print sysroot");
    let untrimmed = String::from_utf8_lossy(&sysroot_output.stdout).to_string();
    let sysroot2 = dbg!(untrimmed.trim());
    let mut sysroot = sysroot2.to_string();
    sysroot.push('/');
    dbg!(&sysroot);

    let sysroot_path = dbg!(PathBuf::from(sysroot));
    //let sysroot_path = sysroot_path.join("/");

    dbg!(&sysroot_path);

    assert!(
        sysroot_path.exists(),
        "{}",
        format!("sysroot path '{}' not found!", sysroot_path.display())
    );

    dbg!(&sysroot_path);

    let bin_dir = sysroot_path.join("bin");
    dbg!(&bin_dir);
    //  ^ this is the dir we want to copy our clippy binary into now

    // there should not be
    std::fs::read_dir(&clippy_exec_dir)
        .expect("failed to read clippys target/ dir")
        .map(|entry| entry.ok().expect("could not convert direntry into path"))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .for_each(|file| {
            let old_path = dbg!(file.clone());
            let new_base = dbg!(PathBuf::from(&bin_dir));
            let bin_file_name = dbg!(old_path.parent().unwrap());
            let new_path = dbg!(new_base.with_file_name(&bin_file_name));

            fs::copy(dbg!(old_path), dbg!(new_path)).expect("could not copy files"); //error

            //   https://github.com/rust-lang/rust-clippy/actions/runs/5700035285/job/15449530554#step:8:132
        });
    let output = dbg!(
        Command::new("python")
            .arg("./x.py")
            .current_dir(&repo_dir)
            .env("RUST_BACKTRACE", "full")
            .env("PATH", new_path)
            .args(["clippy", "-Wclippy::pedantic", "-Wclippy::nursery"])
    )
    .output()
    .expect("unable to run x.py  clippy");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // debug:
    eprintln!("{stderr}");

    // this is an internal test to make sure we would correctly panic on a delay_span_bug
    if repo_name == "matthiaskrgr/clippy_ci_panic_test" {
        // we need to kind of switch around our logic here:
        // if we find a panic, everything is fine, if we don't panic, SOMETHING is broken about our testing

        // the repo basically just contains a delay_span_bug that forces rustc/clippy to panic:
        /*
           #![feature(rustc_attrs)]
           #[rustc_error(delay_span_bug_from_inside_query)]
           fn main() {}
        */

        if stderr.find("error: internal compiler error").is_some() {
            eprintln!("we saw that we intentionally panicked, yay");
            return;
        }

        panic!("panic caused by delay_span_bug was NOT detected! Something is broken!");
    }

    if let Some(backtrace_start) = stderr.find("error: internal compiler error") {
        static BACKTRACE_END_MSG: &str = "end of query stack";
        let backtrace_end = stderr[backtrace_start..]
            .find(BACKTRACE_END_MSG)
            .expect("end of backtrace not found");

        panic!(
            "internal compiler error\nBacktrace:\n\n{}",
            &stderr[backtrace_start..backtrace_start + backtrace_end + BACKTRACE_END_MSG.len()]
        );
    } else if stderr.contains("query stack during panic") {
        panic!("query stack during panic in the output");
    } else if stderr.contains("E0463") {
        // Encountering E0463 (can't find crate for `x`) did _not_ cause the build to fail in the
        // past. Even though it should have. That's why we explicitly panic here.
        // See PR #3552 and issue #3523 for more background.
        panic!("error: E0463");
    } else if stderr.contains("E0514") {
        panic!("incompatible crate versions");
    } else if stderr.contains("failed to run `rustc` to learn about target-specific information") {
        panic!("couldn't find librustc_driver, consider setting `LD_LIBRARY_PATH`");
    } else {
        assert!(
            !stderr.contains("toolchain") || !stderr.contains("is not installed"),
            "missing required toolchain"
        );
    }

    match output.status.code() {
        Some(0) => println!("Compilation successful"),
        Some(code) => eprintln!("Compilation failed. Exit code: {code}"),
        None => panic!("Process terminated by signal"),
    }
}
