use crate::utils::{ErrAction, cargo_cmd, expect_action, run_exit_on_err};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

pub fn run<'a>(path: &str, edition: &str, args: impl Iterator<Item = &'a String>) {
    let is_file = expect_action(fs::metadata(path), ErrAction::Read, path).is_file();
    if is_file {
        run_exit_on_err(
            "cargo run",
            cargo_cmd()
                .args(["run", "--bin", "clippy-driver", "--"])
                .args(["-L", "./target/debug"])
                .args(["-Z", "no-codegen"])
                .args(["--edition", edition])
                .arg(path)
                .args(args)
                // Prevent rustc from creating `rustc-ice-*` files the console output is enough.
                .env("RUSTC_ICE", "0"),
        );
    } else {
        // FIXME: This should be using `cargo run`, but can't since cargo loads the config from the
        // current directory. Either we need a way to change the current directory between building
        // running `cargo-clippy` as this currently does by running in two steps, or we need a way
        // to change where cargo loads the config.
        // See https://github.com/rust-lang/cargo/issues/9769

        run_exit_on_err("cargo build", cargo_cmd().args(["build", "--bin", "cargo-clippy"]));

        let mut exe = match env::current_exe() {
            Ok(mut exe) => {
                exe.pop();
                exe
            },
            Err(_) => PathBuf::from("target/debug"),
        };
        #[cfg(windows)]
        exe.push("cargo-clippy.exe");
        #[cfg(not(windows))]
        exe.push("cargo-clippy");

        run_exit_on_err(
            "cargo run",
            Command::new(exe)
                .arg("clippy")
                .args(args)
                // Prevent rustc from creating `rustc-ice-*` files the console output is enough.
                .env("RUSTC_ICE", "0")
                .current_dir(path),
        );
    }
}
