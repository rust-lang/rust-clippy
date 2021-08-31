#![feature(test)] // compiletest_rs requires this attribute
#![feature(once_cell)]
#![feature(try_blocks)]

use compiletest_rs as compiletest;
use compiletest_rs::common::Mode as TestMode;

use std::env::{self, remove_var, set_var, var_os};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod cargo;

// whether to run internal tests or not
const RUN_INTERNAL_TESTS: bool = cfg!(feature = "internal-lints");

const DEFAULT_LINT_CONFIG: &str = "-A unused";

fn host_lib() -> PathBuf {
    option_env!("HOST_LIBS").map_or(cargo::CARGO_TARGET_DIR.join(env!("PROFILE")), PathBuf::from)
}

fn clippy_driver_path() -> PathBuf {
    option_env!("CLIPPY_DRIVER_PATH").map_or(cargo::TARGET_LIB.join("clippy-driver"), PathBuf::from)
}

// When we'll want to use `extern crate ..` for a dependency that is used
// both by the crate and the compiler itself, we can't simply pass -L flags
// as we'll get a duplicate matching versions. Instead, disambiguate with
// `--extern dep=path`.
// See https://github.com/rust-lang/rust-clippy/issues/4015.
//
// FIXME: We cannot use `cargo build --message-format=json` to resolve to dependency files.
//        Because it would force-rebuild if the options passed to `build` command is not the same
//        as what we manually pass to `cargo` invocation
fn third_party_crates() -> String {
    use std::collections::HashMap;
    static CRATES: &[&str] = &[
        "clippy_lints",
        "clippy_utils",
        "if_chain",
        "itertools",
        "quote",
        "regex",
        "serde",
        "serde_derive",
        "syn",
    ];
    let dep_dir = cargo::TARGET_LIB.join("deps");
    let mut crates: HashMap<&str, Vec<PathBuf>> = HashMap::with_capacity(CRATES.len());
    let mut flags = String::new();
    for entry in fs::read_dir(dep_dir).unwrap().flatten() {
        let path = entry.path();
        if let Some(name) = try {
            let name = path.file_name()?.to_str()?;
            let (name, _) = name.strip_suffix(".rlib")?.strip_prefix("lib")?.split_once('-')?;
            CRATES.iter().copied().find(|&c| c == name)?
        } {
            flags += &format!(" --extern {}={}", name, path.display());
            crates.entry(name).or_default().push(path.clone());
        }
    }
    crates.retain(|_, paths| paths.len() > 1);
    if !crates.is_empty() {
        let crate_names = crates.keys().map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ");
        // add backslashes for an easy copy-paste `rm` command
        let paths = crates
            .into_values()
            .flatten()
            .map(|p| strip_current_dir(&p).display().to_string())
            .collect::<Vec<_>>()
            .join(" \\\n");
        // Check which action should be done in order to remove compiled deps.
        // If pre-installed version of compiler is used, `cargo clean` will do.
        // Otherwise (for bootstrapped compiler), the dependencies directory
        // must be removed manually.
        let suggested_action = if std::env::var_os("RUSTC_BOOTSTRAP").is_some() {
            "removing the stageN-tools directory"
        } else {
            "running `cargo clean`"
        };

        panic!(
            "\n----------------------------------------------------------------------\n\
            ERROR: Found multiple rlibs for crates: {}\n\
            Try {} or remove the following files:\n\n{}\n\n\
            For details on this error see https://github.com/rust-lang/rust-clippy/issues/7343\n\
            ----------------------------------------------------------------------\n",
            crate_names, suggested_action, paths
        );
    }
    flags
}

fn default_config() -> compiletest::Config {
    let mut config = compiletest::Config::default();

    if let Ok(filters) = env::var("TESTNAME") {
        config.filters = filters.split(',').map(std::string::ToString::to_string).collect();
    }

    if let Some(path) = option_env!("RUSTC_LIB_PATH") {
        let path = PathBuf::from(path);
        config.run_lib_path = path.clone();
        config.compile_lib_path = path;
    }

    config.target_rustcflags = Some(format!(
        "--emit=metadata -L {0} -L {1} -Dwarnings -Zui-testing {2} {3}",
        host_lib().join("deps").display(),
        cargo::TARGET_LIB.join("deps").display(),
        third_party_crates(),
        DEFAULT_LINT_CONFIG,
    ));

    config.build_base = host_lib().join("test_build_base");
    config.rustc_path = clippy_driver_path();
    config
}

fn run_ui(cfg: &mut compiletest::Config) {
    cfg.mode = TestMode::Ui;
    cfg.src_base = Path::new("tests").join("ui");
    // use tests/clippy.toml
    let _g = VarGuard::set("CARGO_MANIFEST_DIR", std::fs::canonicalize("tests").unwrap());
    compiletest::run_tests(cfg);
}

fn run_internal_tests(cfg: &mut compiletest::Config) {
    // only run internal tests with the internal-tests feature
    if !RUN_INTERNAL_TESTS {
        return;
    }
    cfg.mode = TestMode::Ui;
    cfg.src_base = Path::new("tests").join("ui-internal");
    compiletest::run_tests(cfg);
}

fn run_ui_toml(config: &mut compiletest::Config) {
    fn run_tests(config: &compiletest::Config, mut tests: Vec<tester::TestDescAndFn>) -> Result<bool, io::Error> {
        let mut result = true;
        let opts = compiletest::test_opts(config);
        for dir in fs::read_dir(&config.src_base)? {
            let dir = dir?;
            if !dir.file_type()?.is_dir() {
                continue;
            }
            let dir_path = dir.path();
            let _g = VarGuard::set("CARGO_MANIFEST_DIR", &dir_path);
            for file in fs::read_dir(&dir_path)? {
                let file = file?;
                let file_path = file.path();
                if file.file_type()?.is_dir() {
                    continue;
                }
                if file_path.extension() != Some(OsStr::new("rs")) {
                    continue;
                }
                let paths = compiletest::common::TestPaths {
                    file: file_path,
                    base: config.src_base.clone(),
                    relative_dir: dir_path.file_name().unwrap().into(),
                };
                let test_name = compiletest::make_test_name(config, &paths);
                let index = tests
                    .iter()
                    .position(|test| test.desc.name == test_name)
                    .expect("The test should be in there");
                result &= tester::run_tests_console(&opts, vec![tests.swap_remove(index)])?;
            }
        }
        Ok(result)
    }

    config.mode = TestMode::Ui;
    config.src_base = Path::new("tests").join("ui-toml").canonicalize().unwrap();

    let tests = compiletest::make_tests(config);

    let res = run_tests(config, tests);
    match res {
        Ok(true) => {},
        Ok(false) => panic!("Some tests failed"),
        Err(e) => {
            panic!("I/O failure during tests: {:?}", e);
        },
    }
}

fn run_ui_cargo(config: &mut compiletest::Config) {
    fn run_tests(
        config: &compiletest::Config,
        filters: &[String],
        mut tests: Vec<tester::TestDescAndFn>,
    ) -> Result<bool, io::Error> {
        let mut result = true;
        let opts = compiletest::test_opts(config);

        for dir in fs::read_dir(&config.src_base)? {
            let dir = dir?;
            if !dir.file_type()?.is_dir() {
                continue;
            }

            // Use the filter if provided
            let dir_path = dir.path();
            for filter in filters {
                if !dir_path.ends_with(filter) {
                    continue;
                }
            }

            for case in fs::read_dir(&dir_path)? {
                let case = case?;
                if !case.file_type()?.is_dir() {
                    continue;
                }

                let src_path = case.path().join("src");

                // When switching between branches, if the previous branch had a test
                // that the current branch does not have, the directory is not removed
                // because an ignored Cargo.lock file exists.
                if !src_path.exists() {
                    continue;
                }

                env::set_current_dir(&src_path)?;
                for file in fs::read_dir(&src_path)? {
                    let file = file?;
                    if file.file_type()?.is_dir() {
                        continue;
                    }

                    // Search for the main file to avoid running a test for each file in the project
                    let file_path = file.path();
                    match file_path.file_name().and_then(OsStr::to_str) {
                        Some("main.rs") => {},
                        _ => continue,
                    }
                    let _g = VarGuard::set("CLIPPY_CONF_DIR", case.path());
                    let paths = compiletest::common::TestPaths {
                        file: file_path,
                        base: config.src_base.clone(),
                        relative_dir: src_path.strip_prefix(&config.src_base).unwrap().into(),
                    };
                    let test_name = compiletest::make_test_name(config, &paths);
                    let index = tests
                        .iter()
                        .position(|test| test.desc.name == test_name)
                        .expect("The test should be in there");
                    result &= tester::run_tests_console(&opts, vec![tests.swap_remove(index)])?;
                }
            }
        }
        Ok(result)
    }

    if cargo::is_rustc_test_suite() {
        return;
    }

    config.mode = TestMode::Ui;
    config.src_base = Path::new("tests").join("ui-cargo").canonicalize().unwrap();

    let tests = compiletest::make_tests(config);

    let current_dir = env::current_dir().unwrap();
    let res = run_tests(config, &config.filters, tests);
    env::set_current_dir(current_dir).unwrap();

    match res {
        Ok(true) => {},
        Ok(false) => panic!("Some tests failed"),
        Err(e) => {
            panic!("I/O failure during tests: {:?}", e);
        },
    }
}

fn prepare_env() {
    set_var("CLIPPY_DISABLE_DOCS_LINKS", "true");
    set_var("__CLIPPY_INTERNAL_TESTS", "true");
    //set_var("RUST_BACKTRACE", "0");
}

#[test]
fn compile_test() {
    prepare_env();
    let mut config = default_config();
    run_ui(&mut config);
    run_ui_toml(&mut config);
    run_ui_cargo(&mut config);
    run_internal_tests(&mut config);
}

/// Restores an env var on drop
#[must_use]
struct VarGuard {
    key: &'static str,
    value: Option<OsString>,
}

impl VarGuard {
    fn set(key: &'static str, val: impl AsRef<OsStr>) -> Self {
        let value = var_os(key);
        set_var(key, val);
        Self { key, value }
    }
}

impl Drop for VarGuard {
    fn drop(&mut self) {
        match self.value.as_deref() {
            None => remove_var(self.key),
            Some(value) => set_var(self.key, value),
        }
    }
}

fn strip_current_dir(path: &Path) -> &Path {
    if let Ok(curr) = env::current_dir() {
        if let Ok(stripped) = path.strip_prefix(curr) {
            return stripped;
        }
    }
    path
}
