//! Read configurations files.

#![deny(clippy::missing_docs_in_private_items)]

mod helpers;

use lazy_static::lazy_static;
use std::sync::Mutex;
use std::{env, fmt, fs, io, mem, path};
use syntax::ast::{LitKind, MetaItemKind, NestedMetaItem};
use syntax::source_map;

pub use self::helpers::Conf;

/// Gets the configuration file from arguments.
pub fn file_from_args(args: &[NestedMetaItem]) -> Result<Option<path::PathBuf>, (&'static str, source_map::Span)> {
    for arg in args.iter().filter_map(NestedMetaItem::meta_item) {
        if arg.check_name(sym!(conf_file)) {
            return match arg.kind {
                MetaItemKind::Word | MetaItemKind::List(_) => Err(("`conf_file` must be a named value", arg.span)),
                MetaItemKind::NameValue(ref value) => {
                    if let LitKind::Str(ref file, _) = value.kind {
                        Ok(Some(file.to_string().into()))
                    } else {
                        Err(("`conf_file` value must be a string", value.span))
                    }
                },
            };
        }
    }

    Ok(None)
}

/// Error from reading a configuration file.
#[derive(Debug)]
pub enum Error {
    /// An I/O error.
    Io(io::Error),
    /// Not valid toml or doesn't fit the expected conf format
    Toml(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Io(ref err) => err.fmt(f),
            Self::Toml(ref err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

lazy_static! {
    static ref ERRORS: Mutex<Vec<Error>> = Mutex::new(Vec::new());
}

/// Search for the configuration file.
pub fn lookup_conf_file() -> io::Result<Option<path::PathBuf>> {
    /// Possible filename to search for.
    const CONFIG_FILE_NAMES: [&str; 2] = [".clippy.toml", "clippy.toml"];

    // Start looking for a config file in CLIPPY_CONF_DIR, or failing that, CARGO_MANIFEST_DIR.
    // If neither of those exist, use ".".
    let mut current = path::PathBuf::from(
        env::var("CLIPPY_CONF_DIR")
            .or_else(|_| env::var("CARGO_MANIFEST_DIR"))
            .unwrap_or_else(|_| ".".to_string()),
    );
    loop {
        for config_file_name in &CONFIG_FILE_NAMES {
            let config_file = current.join(config_file_name);
            match fs::metadata(&config_file) {
                // Only return if it's a file to handle the unlikely situation of a directory named
                // `clippy.toml`.
                Ok(ref md) if md.is_file() => return Ok(Some(config_file)),
                // Return the error if it's something other than `NotFound`; otherwise we didn't
                // find the project file yet, and continue searching.
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {},
            }
        }

        // If the current directory has no parent, we're done searching.
        if !current.pop() {
            return Ok(None);
        }
    }
}

/// Produces a `Conf` filled with the default values and forwards the errors
///
/// Used internally for convenience
fn default(errors: Vec<Error>) -> (Conf, Vec<Error>) {
    (Conf::default(), errors)
}

/// Read the `toml` configuration file.
///
/// In case of error, the function tries to continue as much as possible.
pub fn read(path: &path::Path) -> (Conf, Vec<Error>) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => return default(vec![err.into()]),
    };

    let mut errors = ERRORS.lock().expect("no threading -> mutex always safe");
    assert!(errors.is_empty());

    let mut errors = mem::take(&mut *errors);

    match toml::from_str(&content) {
        Ok(toml) => {
            let toml_ref: &Conf = &toml;

            let cyc_field: Option<u64> = toml_ref.cyclomatic_complexity_threshold;

            if cyc_field.is_some() {
                let cyc_err = "found deprecated field `cyclomatic-complexity-threshold`. Please use `cognitive-complexity-threshold` instead.".to_string();
                errors.push(Error::Toml(cyc_err));
            }

            (toml, errors)
        },
        Err(e) => {
            errors.push(Error::Toml(e.to_string()));

            default(errors)
        },
    }
}
