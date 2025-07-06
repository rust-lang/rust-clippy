#![allow(rustc::diagnostic_outside_of_impl)]
#![allow(rustc::untranslatable_diagnostic)]
#![feature(rustc_private)]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]
// warn on rustc internal lints
#![warn(rustc::internal)]

// FIXME: switch to something more ergonomic here, once available.
// (Currently there is no way to opt into sysroot crates without `extern crate`.)
extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_session;
extern crate rustc_span;

// See docs in https://github.com/rust-lang/rust/blob/master/compiler/rustc/src/main.rs
// about jemalloc.
#[cfg(feature = "jemalloc")]
extern crate tikv_jemalloc_sys as jemalloc_sys;

use clippy_utils::sym;
use declare_clippy_lint::LintListBuilder;
use rustc_interface::interface;
use rustc_session::EarlyDiagCtxt;
use rustc_session::config::ErrorOutputType;
use rustc_session::parse::ParseSess;
use rustc_span::symbol::Symbol;

use std::env;
use std::fs::read_to_string;
use std::path::Path;
use std::process::exit;

use anstream::println;

/// If a command-line option matches `find_arg`, then apply the predicate `pred` on its value. If
/// true, then return it. The parameter is assumed to be either `--arg=value` or `--arg value`.
fn arg_value<'a>(args: &'a [String], find_arg: &str, pred: impl Fn(&str) -> bool) -> Option<&'a str> {
    let mut args = args.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let mut arg = arg.splitn(2, '=');
        if arg.next() != Some(find_arg) {
            continue;
        }

        match arg.next().or_else(|| args.next()) {
            Some(v) if pred(v) => return Some(v),
            _ => {},
        }
    }
    None
}

fn inject_lint_args_from_config(clippy_args: &mut Vec<String>) {
    // Load merged configuration from both Cargo.toml and clippy.toml
    let merged_config = load_merged_lint_config();
    // Collect all lints with their priorities for sorting
    let mut all_lints: Vec<(String, String, i64)> = Vec::new();

    // Collect rust lints
    for (lint_name, (level_str, priority, _source)) in &merged_config.rust_lints {
        all_lints.push((
            format!("--{}={}", level_to_flag(level_str), lint_name),
            level_str.to_string(),
            *priority,
        ));
    }

    // Collect clippy lints
    for (lint_name, (level_str, priority, _source)) in &merged_config.clippy_lints {
        all_lints.push((
            format!("--{}=clippy::{}", level_to_flag(level_str), lint_name),
            level_str.to_string(),
            *priority,
        ));
    }

    // Sort by priority (higher priority first, then by lint name for stability)
    all_lints.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    // Add sorted arguments to clippy_args
    for (arg, _level, _priority) in all_lints {
        clippy_args.push(arg);
    }
}

fn load_merged_lint_config() -> clippy_config::MergedLintConfig {
    clippy_config::MergedLintConfig::load_static()
}

fn level_to_flag(level: &str) -> &'static str {
    if level.eq_ignore_ascii_case("allow") {
        "allow"
    } else if level.eq_ignore_ascii_case("warn") {
        "warn"
    } else if level.eq_ignore_ascii_case("deny") {
        "deny"
    } else if level.eq_ignore_ascii_case("forbid") {
        "forbid"
    } else {
        "warn" // default to warn for unknown levels
    }
}

fn has_arg(args: &[String], find_arg: &str) -> bool {
    args.iter().any(|arg| find_arg == arg.split('=').next().unwrap())
}

fn track_clippy_args(psess: &mut ParseSess, args_env_var: Option<&str>) {
    psess
        .env_depinfo
        .get_mut()
        .insert((sym::CLIPPY_ARGS, args_env_var.map(Symbol::intern)));
}

/// Track files that may be accessed at runtime in `file_depinfo` so that cargo will re-run clippy
/// when any of them are modified
fn track_files(psess: &mut ParseSess) {
    let file_depinfo = psess.file_depinfo.get_mut();

    // Used by `clippy::cargo` lints and to determine the MSRV. `cargo clippy` executes `clippy-driver`
    // with the current directory set to `CARGO_MANIFEST_DIR` so a relative path is fine
    if Path::new("Cargo.toml").exists() {
        file_depinfo.insert(sym::Cargo_toml);
    }

    // `clippy.toml` will be automatically tracked as it's loaded with `sess.source_map().load_file()`

    // During development track the `clippy-driver` executable so that cargo will re-run clippy whenever
    // it is rebuilt
    if cfg!(debug_assertions)
        && let Ok(current_exe) = env::current_exe()
        && let Some(current_exe) = current_exe.to_str()
    {
        file_depinfo.insert(Symbol::intern(current_exe));
    }
}

struct DefaultCallbacks;
impl rustc_driver::Callbacks for DefaultCallbacks {}

/// This is different from `DefaultCallbacks` that it will inform Cargo to track the value of
/// `CLIPPY_ARGS` environment variable.
struct RustcCallbacks {
    clippy_args_var: Option<String>,
}

impl rustc_driver::Callbacks for RustcCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        let clippy_args_var = self.clippy_args_var.take();
        config.psess_created = Some(Box::new(move |psess| {
            track_clippy_args(psess, clippy_args_var.as_deref());
        }));
    }
}

struct ClippyCallbacks {
    clippy_args_var: Option<String>,
}

impl rustc_driver::Callbacks for ClippyCallbacks {
    // JUSTIFICATION: necessary in clippy driver to set `mir_opt_level`
    #[allow(rustc::bad_opt_access)]
    fn config(&mut self, config: &mut interface::Config) {
        let conf_path = clippy_config::lookup_conf_file();
        let previous = config.register_lints.take();
        let clippy_args_var = self.clippy_args_var.take();
        config.psess_created = Some(Box::new(move |psess| {
            track_clippy_args(psess, clippy_args_var.as_deref());
            track_files(psess);

            // Trigger a rebuild if CLIPPY_CONF_DIR changes. The value must be a valid string so
            // changes between dirs that are invalid UTF-8 will not trigger rebuilds
            psess.env_depinfo.get_mut().insert((
                sym::CLIPPY_CONF_DIR,
                env::var("CLIPPY_CONF_DIR").ok().map(|dir| Symbol::intern(&dir)),
            ));
        }));
        config.register_lints = Some(Box::new(move |sess, lint_store| {
            // technically we're ~guaranteed that this is none but might as well call anything that
            // is there already. Certainly it can't hurt.
            if let Some(previous) = &previous {
                (previous)(sess, lint_store);
            }

            let mut list_builder = LintListBuilder::default();
            list_builder.insert(clippy_lints::declared_lints::LINTS);
            list_builder.register(lint_store);

            let conf = clippy_config::Conf::read(sess, &conf_path);
            clippy_lints::register_lint_passes(lint_store, conf);

            #[cfg(feature = "internal")]
            clippy_lints_internal::register_lints(lint_store);
        }));
        config.extra_symbols = sym::EXTRA_SYMBOLS.into();

        // FIXME: #4825; This is required, because Clippy lints that are based on MIR have to be
        // run on the unoptimized MIR. On the other hand this results in some false negatives. If
        // MIR passes can be enabled / disabled separately, we should figure out, what passes to
        // use for Clippy.
        config.opts.unstable_opts.mir_opt_level = Some(0);
        config.opts.unstable_opts.mir_enable_passes =
            vec![("CheckNull".to_owned(), false), ("CheckAlignment".to_owned(), false)];

        // Disable flattening and inlining of format_args!(), so the HIR matches with the AST.
        config.opts.unstable_opts.flatten_format_args = false;
    }
}

#[allow(clippy::ignored_unit_patterns)]
fn display_help() {
    println!("{}", help_message());
}

const BUG_REPORT_URL: &str = "https://github.com/rust-lang/rust-clippy/issues/new?template=ice.yml";

#[allow(clippy::too_many_lines)]
#[allow(clippy::ignored_unit_patterns)]
pub fn main() {
    // See docs in https://github.com/rust-lang/rust/blob/master/compiler/rustc/src/main.rs
    // about jemalloc.
    #[cfg(feature = "jemalloc")]
    {
        use std::os::raw::{c_int, c_void};

        #[used]
        static _F1: unsafe extern "C" fn(usize, usize) -> *mut c_void = jemalloc_sys::calloc;
        #[used]
        static _F2: unsafe extern "C" fn(*mut *mut c_void, usize, usize) -> c_int = jemalloc_sys::posix_memalign;
        #[used]
        static _F3: unsafe extern "C" fn(usize, usize) -> *mut c_void = jemalloc_sys::aligned_alloc;
        #[used]
        static _F4: unsafe extern "C" fn(usize) -> *mut c_void = jemalloc_sys::malloc;
        #[used]
        static _F5: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void = jemalloc_sys::realloc;
        #[used]
        static _F6: unsafe extern "C" fn(*mut c_void) = jemalloc_sys::free;

        #[cfg(target_os = "macos")]
        {
            unsafe extern "C" {
                fn _rjem_je_zone_register();
            }

            #[used]
            static _F7: unsafe extern "C" fn() = _rjem_je_zone_register;
        }
    }

    let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());

    rustc_driver::init_rustc_env_logger(&early_dcx);

    rustc_driver::install_ice_hook(BUG_REPORT_URL, |dcx| {
        // FIXME: this macro calls unwrap internally but is called in a panicking context!  It's not
        // as simple as moving the call from the hook to main, because `install_ice_hook` doesn't
        // accept a generic closure.
        let version_info = rustc_tools_util::get_version_info!();
        dcx.handle().note(format!("Clippy version: {version_info}"));
    });

    exit(rustc_driver::catch_with_exit_code(move || {
        let mut orig_args = rustc_driver::args::raw_args(&early_dcx);

        let has_sysroot_arg = |args: &mut [String]| -> bool {
            if has_arg(args, "--sysroot") {
                return true;
            }
            // https://doc.rust-lang.org/rustc/command-line-arguments.html#path-load-command-line-flags-from-a-path
            // Beside checking for existence of `--sysroot` on the command line, we need to
            // check for the arg files that are prefixed with @ as well to be consistent with rustc
            for arg in args.iter() {
                if let Some(arg_file_path) = arg.strip_prefix('@')
                    && let Ok(arg_file) = read_to_string(arg_file_path)
                {
                    let split_arg_file: Vec<String> = arg_file.lines().map(ToString::to_string).collect();
                    if has_arg(&split_arg_file, "--sysroot") {
                        return true;
                    }
                }
            }
            false
        };

        let sys_root_env = std::env::var("SYSROOT").ok();
        let pass_sysroot_env_if_given = |args: &mut Vec<String>, sys_root_env| {
            if let Some(sys_root) = sys_root_env
                && !has_sysroot_arg(args)
            {
                args.extend(vec!["--sysroot".into(), sys_root]);
            }
        };

        // make "clippy-driver --rustc" work like a subcommand that passes further args to "rustc"
        // for example `clippy-driver --rustc --version` will print the rustc version that clippy-driver
        // uses
        if let Some(pos) = orig_args.iter().position(|arg| arg == "--rustc") {
            orig_args.remove(pos);
            orig_args[0] = "rustc".to_string();

            let mut args: Vec<String> = orig_args.clone();
            pass_sysroot_env_if_given(&mut args, sys_root_env);

            rustc_driver::run_compiler(&args, &mut DefaultCallbacks);
            return;
        }

        if orig_args.iter().any(|a| a == "--version" || a == "-V") {
            let version_info = rustc_tools_util::get_version_info!();

            println!("{version_info}");
            exit(0);
        }

        // Setting RUSTC_WRAPPER causes Cargo to pass 'rustc' as the first argument.
        // We're invoking the compiler programmatically, so we ignore this/
        let wrapper_mode = orig_args.get(1).map(Path::new).and_then(Path::file_stem) == Some("rustc".as_ref());

        if wrapper_mode {
            // we still want to be able to invoke it normally though
            orig_args.remove(1);
        }

        if !wrapper_mode && (orig_args.iter().any(|a| a == "--help" || a == "-h") || orig_args.len() == 1) {
            display_help();
            exit(0);
        }

        let mut args: Vec<String> = orig_args.clone();
        pass_sysroot_env_if_given(&mut args, sys_root_env);

        let mut no_deps = false;
        let clippy_args_var = env::var("CLIPPY_ARGS").ok();
        let mut clippy_args = clippy_args_var
            .as_deref()
            .unwrap_or_default()
            .split("__CLIPPY_HACKERY__")
            .filter_map(|s| match s {
                "" => None,
                "--no-deps" => {
                    no_deps = true;
                    None
                },
                _ => Some(s.to_string()),
            })
            .chain(vec!["--cfg".into(), "clippy".into()])
            .collect::<Vec<String>>();

        // Load lint configurations from both Cargo.toml and clippy.toml and add them as arguments
        inject_lint_args_from_config(&mut clippy_args);

        // If no Clippy lints will be run we do not need to run Clippy
        let cap_lints_allow = arg_value(&orig_args, "--cap-lints", |val| val == "allow").is_some()
            && arg_value(&orig_args, "--force-warn", |val| val.contains("clippy::")).is_none();

        // If `--no-deps` is enabled only lint the primary package
        let relevant_package = !no_deps || env::var("CARGO_PRIMARY_PACKAGE").is_ok();

        // Do not run Clippy for Cargo's info queries so that invalid CLIPPY_ARGS are not cached
        // https://github.com/rust-lang/cargo/issues/14385
        let info_query = has_arg(&orig_args, "-vV") || has_arg(&orig_args, "--print");

        let clippy_enabled = !cap_lints_allow && relevant_package && !info_query;
        if clippy_enabled {
            args.extend(clippy_args);
            rustc_driver::run_compiler(&args, &mut ClippyCallbacks { clippy_args_var });
        } else {
            rustc_driver::run_compiler(&args, &mut RustcCallbacks { clippy_args_var });
        }
    }))
}

#[must_use]
fn help_message() -> &'static str {
    color_print::cstr!(
        "Checks a file to catch common mistakes and improve your Rust code.
Run <cyan>clippy-driver</> with the same arguments you use for <cyan>rustc</>

<green,bold>Usage</>:
    <cyan,bold>clippy-driver</> <cyan>[OPTIONS] INPUT</>

<green,bold>Common options:</>
    <cyan,bold>-h</>, <cyan,bold>--help</>               Print this message
    <cyan,bold>-V</>, <cyan,bold>--version</>            Print version info and exit
    <cyan,bold>--rustc</>                  Pass all arguments to <cyan>rustc</>

<green,bold>Allowing / Denying lints</>
You can use tool lints to allow or deny lints from your code, e.g.:

    <yellow,bold>#[allow(clippy::needless_lifetimes)]</>
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_to_flag() {
        assert_eq!(level_to_flag("allow"), "allow");
        assert_eq!(level_to_flag("ALLOW"), "allow");
        assert_eq!(level_to_flag("warn"), "warn");
        assert_eq!(level_to_flag("WARN"), "warn");
        assert_eq!(level_to_flag("deny"), "deny");
        assert_eq!(level_to_flag("forbid"), "forbid");
        assert_eq!(level_to_flag("unknown"), "warn"); // Default
    }

    #[test]
    fn test_inject_lint_args_priority_sorting() {
        // Create a mock merged config for testing
        let mut rust_lints = std::collections::BTreeMap::new();
        rust_lints.insert(
            "dead_code".to_string(),
            ("allow".to_string(), 5, Some("clippy.toml".to_string())),
        );
        rust_lints.insert(
            "unused_variables".to_string(),
            ("warn".to_string(), 10, Some("Cargo.toml".to_string())),
        );
        rust_lints.insert(
            "unused_imports".to_string(),
            ("deny".to_string(), 1, Some("clippy.toml".to_string())),
        );

        let mut clippy_lints = std::collections::BTreeMap::new();
        clippy_lints.insert(
            "needless_return".to_string(),
            ("allow".to_string(), 15, Some("clippy.toml".to_string())),
        );
        clippy_lints.insert(
            "single_match".to_string(),
            ("warn".to_string(), 5, Some("Cargo.toml".to_string())),
        );
        clippy_lints.insert(
            "too_many_arguments".to_string(),
            ("forbid".to_string(), 0, Some("clippy.toml".to_string())),
        );

        // Simulate the sorting behavior
        let mut all_lints: Vec<(String, String, i64)> = Vec::new();

        for (lint_name, (level_str, priority, _source)) in &rust_lints {
            all_lints.push((
                format!("--{}={}", level_to_flag(level_str), lint_name),
                level_str.clone(),
                *priority,
            ));
        }

        for (lint_name, (level_str, priority, _source)) in &clippy_lints {
            all_lints.push((
                format!("--{}=clippy::{}", level_to_flag(level_str), lint_name),
                level_str.clone(),
                *priority,
            ));
        }

        // Sort by priority (higher priority first, then by lint name for stability)
        all_lints.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

        // Extract just the arguments
        let args: Vec<String> = all_lints.into_iter().map(|(arg, _, _)| arg).collect();

        // Verify the expected order (highest priority first, then alphabetical within same priority)
        let expected = vec![
            "--allow=clippy::needless_return",     // priority 15
            "--warn=unused_variables",             // priority 10
            "--allow=dead_code",                   // priority 5 (first alphabetically among priority 5)
            "--warn=clippy::single_match",         // priority 5 (second alphabetically among priority 5)
            "--deny=unused_imports",               // priority 1
            "--forbid=clippy::too_many_arguments", // priority 0
        ];

        assert_eq!(args, expected);
    }

    #[test]
    fn test_arg_value() {
        let args = vec![
            "--cap-lints=allow".to_string(),
            "--force-warn".to_string(),
            "clippy::needless_return".to_string(),
            "--other-flag=value".to_string(),
        ];

        assert_eq!(arg_value(&args, "--cap-lints", |val| val == "allow"), Some("allow"));
        assert_eq!(arg_value(&args, "--cap-lints", |val| val == "warn"), None);
        assert_eq!(
            arg_value(&args, "--force-warn", |val| val.contains("clippy::")),
            Some("clippy::needless_return")
        );
        assert_eq!(arg_value(&args, "--nonexistent", |_| true), None);
    }

    #[test]
    fn test_has_arg() {
        let args = vec![
            "--cap-lints=allow".to_string(),
            "--version".to_string(),
            "--help".to_string(),
        ];

        assert!(has_arg(&args, "--cap-lints"));
        assert!(has_arg(&args, "--version"));
        assert!(has_arg(&args, "--help"));
        assert!(!has_arg(&args, "--nonexistent"));
    }
}
