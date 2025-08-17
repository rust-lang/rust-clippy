use crate::utils::{ErrAction, cargo_cmd, expect_action};
use core::fmt::Display;
use core::mem;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};
<<<<<<< HEAD
use std::{fs, thread};
use walkdir::WalkDir;
=======
use std::{env, thread};
use std::io;
>>>>>>> 48fa2fac9 (Refactor run and mtime functions: safer error handling, cleaner structure)

/// Python binary depending on OS
#[cfg(windows)]
const PYTHON: &str = "python";
#[cfg(not(windows))]
const PYTHON: &str = "python3";

/// Run a local live server for Clippy docs and lints.
///
/// # Arguments
/// * `port` - Port to run the HTTP server on.
/// * `lint` - Optional lint name to open directly in the browser.
///
/// # Panics
/// Panics if spawning processes fails or the HTTP server cannot be launched.
pub fn run(port: u16, lint: Option<String>) -> ! {
    let mut url = Some(match lint {
        Some(lint) => format!("http://localhost:{port}/#{lint}"),
        None => format!("http://localhost:{port}"),
    });

<<<<<<< HEAD
    let mut last_update = mtime("util/gh-pages/index.html");
    loop {
        if is_metadata_outdated(mem::replace(&mut last_update, SystemTime::now())) {
            // Ignore the command result; we'll fall back to displaying the old metadata.
            let _ = expect_action(
                cargo_cmd().arg("collect-metadata").status(),
                ErrAction::Run,
                "cargo collect-metadata",
            );
            last_update = SystemTime::now();
        }

        // Only start the web server the first time around.
        if let Some(url) = url.take() {
            thread::spawn(move || {
                let mut child = expect_action(
                    Command::new(PYTHON)
                        .args(["-m", "http.server", port.to_string().as_str()])
                        .current_dir("util/gh-pages")
                        .spawn(),
                    ErrAction::Run,
                    "python -m http.server",
                );
                // Give some time for python to start
                thread::sleep(Duration::from_millis(500));
                // Launch browser after first export.py has completed and http.server is up
                let _result = opener::open(url);
                expect_action(child.wait(), ErrAction::Run, "python -m http.server");
            });
        }

        // Delay to avoid updating the metadata too aggressively.
=======
    let mut server_started = false;

    loop {
        // Check last modified times of critical files
        let index_time = mtime("util/gh-pages/index.html").unwrap_or(SystemTime::UNIX_EPOCH);
        let times = [
            "clippy_lints/src",
            "util/gh-pages/index_template.html",
            "tests/compile-test.rs",
        ]
        .iter()
        .filter_map(|p| mtime(p).ok())
        .collect::<Vec<_>>();

        // Rebuild metadata if any file is newer than index.html
        if times.iter().any(|&time| index_time < time) {
            Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                .arg("collect-metadata")
                .spawn()
                .expect("Failed to spawn cargo collect-metadata process")
                .wait()
                .expect("Cargo collect-metadata process failed");
        }

        // Start HTTP server and open browser once
        if !server_started {
            if let Some(url) = url.take() {
                thread::spawn(move || {
                    let mut child = Command::new(PYTHON)
                        .arg("-m")
                        .arg("http.server")
                        .arg(port.to_string())
                        .current_dir("util/gh-pages")
                        .spawn()
                        .expect("Failed to spawn Python HTTP server");

                    // Wait until server starts
                    thread::sleep(Duration::from_millis(500));

                    // Open browser after first export
                    let _ = opener::open(url);

                    child.wait().expect("Python HTTP server process failed");
                });
                server_started = true;
            }
        }

>>>>>>> 48fa2fac9 (Refactor run and mtime functions: safer error handling, cleaner structure)
        thread::sleep(Duration::from_millis(1000));
    }
}

<<<<<<< HEAD
fn log_err_and_continue<T>(res: Result<T, impl Display>, path: &Path) -> Option<T> {
    match res {
        Ok(x) => Some(x),
        Err(ref e) => {
            eprintln!("error reading `{}`: {e}", path.display());
            None
        },
=======
/// Get the most recent modification time of a file or directory recursively.
/// Returns `io::Result<SystemTime>`.
fn mtime(path: impl AsRef<Path>) -> io::Result<SystemTime> {
    let path = path.as_ref();

    if path.is_dir() {
        let mut latest = SystemTime::UNIX_EPOCH;
        for entry in path.read_dir()? {
            let entry = entry?;
            let entry_time = mtime(entry.path())?;
            if entry_time > latest {
                latest = entry_time;
            }
        }
        Ok(latest)
    } else {
        Ok(path.metadata()?.modified()?)
>>>>>>> 48fa2fac9 (Refactor run and mtime functions: safer error handling, cleaner structure)
    }
}

fn mtime(path: &str) -> SystemTime {
    log_err_and_continue(fs::metadata(path), path.as_ref())
        .and_then(|metadata| log_err_and_continue(metadata.modified(), path.as_ref()))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn is_metadata_outdated(time: SystemTime) -> bool {
    // Ignore all IO errors here. We don't want to stop them from hosting the server.
    if time < mtime("util/gh-pages/index_template.html") || time < mtime("tests/compile-test.rs") {
        return true;
    }
    let Some(dir) = log_err_and_continue(fs::read_dir("."), ".".as_ref()) else {
        return false;
    };
    dir.map_while(|e| log_err_and_continue(e, ".".as_ref())).any(|e| {
        let name = e.file_name();
        let name_bytes = name.as_encoded_bytes();
        if (name_bytes.starts_with(b"clippy_lints") && name_bytes != b"clippy_lints_internal")
            || name_bytes == b"clippy_config"
        {
            WalkDir::new(&name)
                .into_iter()
                .map_while(|e| log_err_and_continue(e, name.as_ref()))
                .filter(|e| e.file_type().is_file())
                .filter_map(|e| {
                    log_err_and_continue(e.metadata(), e.path())
                        .and_then(|m| log_err_and_continue(m.modified(), e.path()))
                })
                .any(|ftime| time < ftime)
        } else {
            false
        }
    })
}
