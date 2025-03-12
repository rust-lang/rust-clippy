use std::path::{Path, PathBuf};
use std::process::{self, ExitStatus};
use std::{fs, io};

#[cfg(not(windows))]
static CARGO_CLIPPY_EXE: &str = "cargo-clippy";
#[cfg(windows)]
static CARGO_CLIPPY_EXE: &str = "cargo-clippy.exe";

/// Returns the path to the `cargo-clippy` binary
///
/// # Panics
///
/// Panics if the path of current executable could not be retrieved.
#[must_use]
pub fn cargo_clippy_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current executable name");
    path.set_file_name(CARGO_CLIPPY_EXE);
    path
}

/// Returns the path to the Clippy project directory
///
/// # Panics
///
/// Panics if the current directory could not be retrieved, there was an error reading any of the
/// Cargo.toml files or ancestor directory is the clippy root directory
#[must_use]
pub fn clippy_project_root() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap();
    for path in current_dir.ancestors() {
        let result = fs::read_to_string(path.join("Cargo.toml"));
        if let Err(err) = &result {
            if err.kind() == io::ErrorKind::NotFound {
                continue;
            }
        }

        let content = result.unwrap();
        if content.contains("[package]\nname = \"clippy\"") {
            return path.to_path_buf();
        }
    }
    panic!("error: Can't determine root of project. Please run inside a Clippy working dir.");
}

/// # Panics
/// Panics if given command result was failed.
pub fn exit_if_err(status: io::Result<ExitStatus>) {
    match status.expect("failed to run command").code() {
        Some(0) => {},
        Some(n) => process::exit(n),
        None => {
            eprintln!("Killed by signal");
            process::exit(1);
        },
    }
}

/// Returns the Clippy version as a tuple of minor and patch version numbers.
///
/// This function reads the version from Cargo.toml and parses it into a tuple.
/// The major version is assumed to be 0, and only minor and patch are returned.
pub(crate) fn clippy_version() -> (u32, u32) {
    /// Parses a Cargo.toml file content to extract the minor and patch version numbers.
    ///
    /// # Arguments
    ///
    /// * `contents` - The string content of the Cargo.toml file
    ///
    /// # Returns
    ///
    /// * `Option<(u32, u32)>` - The minor and patch version numbers if parsing succeeds
    fn parse_manifest(contents: &str) -> Option<(u32, u32)> {
        // Find the version line in the manifest
        let version = contents
            .lines()
            .filter_map(|l| l.split_once('='))
            .find_map(|(k, v)| (k.trim() == "version").then(|| v.trim()))?;

        // Extract version string and validate it starts with "0."
        let Some(("0", version)) = version.get(1..version.len() - 1)?.split_once('.') else {
            return None;
        };

        // Split and parse minor and patch versions
        let (minor, patch) = version.split_once('.')?;
        Some((minor.parse().ok()?, patch.parse().ok()?))
    }

    // Read Cargo.toml content
    let contents = fs::read_to_string("Cargo.toml").expect("Unable to read `Cargo.toml`");

    // Parse the manifest and extract version
    parse_manifest(&contents).expect("Unable to find package version in `Cargo.toml`")
}

/// Represents the mode for updating lints.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    /// Only check if lints are defined properly without making changes.
    Check,
    /// Update lint definitions.
    Change,
}

/// Exits the program with an error message when lints are not properly defined.
///
/// This function prints an error message instructing the user to run the lint
/// update command and then terminates the program with exit code 1.
pub(crate) fn exit_with_failure() {
    println!(
        "Not all lints defined properly. \
                 Please run `cargo dev update_lints` to make sure all lints are defined properly."
    );
    process::exit(1);
}

/// Replaces a region in a file delimited by two lines matching regexes.
///
/// `path` is the relative path to the file on which you want to perform the replacement.
///
/// See `replace_region_in_text` for documentation of the other options.
///
/// # Panics
///
/// Panics if the path could not read or then written
pub(crate) fn replace_region_in_file(
    update_mode: UpdateMode,
    path: &Path,
    start: &str,
    end: &str,
    write_replacement: impl FnMut(&mut String),
) {
    let contents = fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read from `{}`: {e}", path.display()));
    let new_contents = match replace_region_in_text(&contents, start, end, write_replacement) {
        Ok(x) => x,
        Err(delim) => panic!("Couldn't find `{delim}` in file `{}`", path.display()),
    };

    match update_mode {
        UpdateMode::Check if contents != new_contents => exit_with_failure(),
        UpdateMode::Check => (),
        UpdateMode::Change => {
            if let Err(e) = fs::write(path, new_contents.as_bytes()) {
                panic!("Cannot write to `{}`: {e}", path.display());
            }
        },
    }
}

/// Replaces a region in a text delimited by two strings. Returns the new text if both delimiters
/// were found, or the missing delimiter if not.
pub(crate) fn replace_region_in_text<'a>(
    text: &str,
    start: &'a str,
    end: &'a str,
    mut write_replacement: impl FnMut(&mut String),
) -> Result<String, &'a str> {
    let (text_start, rest) = text.split_once(start).ok_or(start)?;
    let (_, text_end) = rest.split_once(end).ok_or(end)?;

    let mut res = String::with_capacity(text.len() + 4096);
    res.push_str(text_start);
    res.push_str(start);
    write_replacement(&mut res);
    res.push_str(end);
    res.push_str(text_end);

    Ok(res)
}
