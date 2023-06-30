#![feature(rustc_private)]
#![feature(let_chains)]
#![feature(lazy_cell)]
#![feature(lint_reasons)]
#![cfg_attr(feature = "deny-warnings", deny(warnings))]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]
// warn on rustc internal lints
#![warn(rustc::internal)]

// FIXME: switch to something more ergonomic here, once available.
// (Currently there is no way to opt into sysroot crates without `extern crate`.)
extern crate rustc_borrowck;
extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use clippy_lints::{BorrowckContext, BorrowckLintPass};
use rustc_borrowck::consumers::{do_mir_borrowck, BodyWithBorrowckFacts, ConsumerOptions};
use rustc_interface::interface;
use rustc_lint::LintPass;
use rustc_session::parse::ParseSess;
use rustc_span::symbol::Symbol;

use std::env;
use std::ops::Deref;
use std::path::Path;
use std::process::exit;
use std::rc::Rc;

/// If a command-line option matches `find_arg`, then apply the predicate `pred` on its value. If
/// true, then return it. The parameter is assumed to be either `--arg=value` or `--arg value`.
fn arg_value<'a, T: Deref<Target = str>>(
    args: &'a [T],
    find_arg: &str,
    pred: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    let mut args = args.iter().map(Deref::deref);
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

#[test]
fn test_arg_value() {
    let args = &["--bar=bar", "--foobar", "123", "--foo"];

    assert_eq!(arg_value(&[] as &[&str], "--foobar", |_| true), None);
    assert_eq!(arg_value(args, "--bar", |_| false), None);
    assert_eq!(arg_value(args, "--bar", |_| true), Some("bar"));
    assert_eq!(arg_value(args, "--bar", |p| p == "bar"), Some("bar"));
    assert_eq!(arg_value(args, "--bar", |p| p == "foo"), None);
    assert_eq!(arg_value(args, "--foobar", |p| p == "foo"), None);
    assert_eq!(arg_value(args, "--foobar", |p| p == "123"), Some("123"));
    assert_eq!(arg_value(args, "--foobar", |p| p.contains("12")), Some("123"));
    assert_eq!(arg_value(args, "--foo", |_| true), None);
}

fn track_clippy_args(parse_sess: &mut ParseSess, args_env_var: &Option<String>) {
    parse_sess.env_depinfo.get_mut().insert((
        Symbol::intern("CLIPPY_ARGS"),
        args_env_var.as_deref().map(Symbol::intern),
    ));
}

/// Track files that may be accessed at runtime in `file_depinfo` so that cargo will re-run clippy
/// when any of them are modified
fn track_files(parse_sess: &mut ParseSess) {
    let file_depinfo = parse_sess.file_depinfo.get_mut();

    // Used by `clippy::cargo` lints and to determine the MSRV. `cargo clippy` executes `clippy-driver`
    // with the current directory set to `CARGO_MANIFEST_DIR` so a relative path is fine
    if Path::new("Cargo.toml").exists() {
        file_depinfo.insert(Symbol::intern("Cargo.toml"));
    }

    // `clippy.toml` will be automatically tracked as it's loaded with `sess.source_map().load_file()`

    // During development track the `clippy-driver` executable so that cargo will re-run clippy whenever
    // it is rebuilt
    #[expect(
        clippy::collapsible_if,
        reason = "Due to a bug in let_chains this if statement can't be collapsed"
    )]
    if cfg!(debug_assertions) {
        if let Ok(current_exe) = env::current_exe()
            && let Some(current_exe) = current_exe.to_str()
        {
            file_depinfo.insert(Symbol::intern(current_exe));
        }
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
        config.parse_sess_created = Some(Box::new(move |parse_sess| {
            track_clippy_args(parse_sess, &clippy_args_var);
        }));
    }
}

struct ClippyCallbacks {
    clippy_args_var: Option<String>,
}

type BorrowckLintPassObject<'b, 'tcx> = Box<dyn BorrowckLintPass<'b, 'tcx> + 'b>;

/// Like [`rustc_lint::late::RuntimeCombinedLateLintPass`], but with [`BorrowckPass`]es instead of
/// [`rustc_lint::LateLintPass`]es, and the passes are owned instead of borrowed.
pub struct RuntimeCombinedBorrowckLintPass<'b, 'tcx> {
    passes: Vec<BorrowckLintPassObject<'b, 'tcx>>,
}

#[allow(rustc::lint_pass_impl_without_macro)]
impl LintPass for RuntimeCombinedBorrowckLintPass<'_, '_> {
    fn name(&self) -> &'static str {
        panic!()
    }
}

impl<'b, 'tcx> BorrowckLintPass<'b, 'tcx> for RuntimeCombinedBorrowckLintPass<'b, 'tcx> {
    fn check_body_with_facts(
        &mut self,
        cx: &BorrowckContext<'tcx>,
        body_with_facts: &'b Rc<BodyWithBorrowckFacts<'tcx>>,
    ) {
        for pass in &mut self.passes {
            pass.check_body_with_facts(cx, body_with_facts);
        }
    }
}

impl rustc_driver::Callbacks for ClippyCallbacks {
    // JUSTIFICATION: necessary in clippy driver to set `mir_opt_level`
    #[allow(rustc::bad_opt_access)]
    fn config(&mut self, config: &mut interface::Config) {
        let previous = config.register_lints.take();
        let clippy_args_var = self.clippy_args_var.take();
        config.parse_sess_created = Some(Box::new(move |parse_sess| {
            track_clippy_args(parse_sess, &clippy_args_var);
            track_files(parse_sess);
        }));

        config.register_lints = Some(Box::new(move |sess, lint_store| {
            // technically we're ~guaranteed that this is none but might as well call anything that
            // is there already. Certainly it can't hurt.
            if let Some(previous) = &previous {
                (previous)(sess, lint_store);
            }

            let conf = clippy_lints::read_conf(sess);
            clippy_lints::register_plugins(lint_store, sess, conf);
            clippy_lints::register_pre_expansion_lints(lint_store, sess, conf);
            clippy_lints::register_renamed(lint_store);
        }));

        config.override_queries = Some(|_sess, providers, _external_providers| {
            providers.mir_borrowck = |tcx, local_def_id| {
                let (result, body_with_facts) =
                    do_mir_borrowck(tcx, local_def_id, ConsumerOptions::RegionInferenceContext);

                let body_with_facts = Rc::new(*body_with_facts.unwrap());

                let def_id = body_with_facts.body.source.def_id();
                let body_id = tcx.hir().body_owned_by(def_id.as_local().unwrap());

                let cx = BorrowckContext::new(tcx, body_id);

                // FIXME: How to compute this just once?
                let mut pass = {
                    let conf = clippy_lints::read_conf(tcx.sess);
                    let msrv = clippy_utils::msrvs::Msrv::read(&conf.msrv, tcx.sess);

                    let passes = clippy_lints::borrowck_lint_passes(msrv);
    
                    RuntimeCombinedBorrowckLintPass { passes }
                };

                pass.check_body_with_facts(&cx, &body_with_facts);

                tcx.arena.alloc(result)
            };
        });

        // FIXME: #4825; This is required, because Clippy lints that are based on MIR have to be
        // run on the unoptimized MIR. On the other hand this results in some false negatives. If
        // MIR passes can be enabled / disabled separately, we should figure out, what passes to
        // use for Clippy.
        config.opts.unstable_opts.mir_opt_level = Some(0);

        // Disable flattening and inlining of format_args!(), so the HIR matches with the AST.
        config.opts.unstable_opts.flatten_format_args = false;
    }
}

fn display_help() {
    println!(
        "\
Checks a package to catch common mistakes and improve your Rust code.

Usage:
    cargo clippy [options] [--] [<opts>...]

Common options:
    -h, --help               Print this message
        --rustc              Pass all args to rustc
    -V, --version            Print version info and exit

For the other options see `cargo check --help`.

To allow or deny a lint from the command line you can use `cargo clippy --`
with:

    -W --warn OPT       Set lint warnings
    -A --allow OPT      Set lint allowed
    -D --deny OPT       Set lint denied
    -F --forbid OPT     Set lint forbidden

You can use tool lints to allow or deny lints from your code, eg.:

    #[allow(clippy::needless_lifetimes)]
"
    );
}

const BUG_REPORT_URL: &str = "https://github.com/rust-lang/rust-clippy/issues/new";

#[allow(clippy::too_many_lines)]
pub fn main() {
    rustc_driver::init_rustc_env_logger();

    rustc_driver::install_ice_hook(BUG_REPORT_URL, |handler| {
        // FIXME: this macro calls unwrap internally but is called in a panicking context!  It's not
        // as simple as moving the call from the hook to main, because `install_ice_hook` doesn't
        // accept a generic closure.
        let version_info = rustc_tools_util::get_version_info!();
        handler.note_without_error(format!("Clippy version: {version_info}"));
    });

    exit(rustc_driver::catch_with_exit_code(move || {
        let mut orig_args: Vec<String> = env::args().collect();
        let has_sysroot_arg = arg_value(&orig_args, "--sysroot", |_| true).is_some();

        let sys_root_env = std::env::var("SYSROOT").ok();
        let pass_sysroot_env_if_given = |args: &mut Vec<String>, sys_root_env| {
            if let Some(sys_root) = sys_root_env {
                if !has_sysroot_arg {
                    args.extend(vec!["--sysroot".into(), sys_root]);
                }
            };
        };

        // make "clippy-driver --rustc" work like a subcommand that passes further args to "rustc"
        // for example `clippy-driver --rustc --version` will print the rustc version that clippy-driver
        // uses
        if let Some(pos) = orig_args.iter().position(|arg| arg == "--rustc") {
            orig_args.remove(pos);
            orig_args[0] = "rustc".to_string();

            let mut args: Vec<String> = orig_args.clone();
            pass_sysroot_env_if_given(&mut args, sys_root_env);

            return rustc_driver::RunCompiler::new(&args, &mut DefaultCallbacks).run();
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
        let clippy_args = clippy_args_var
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
            .chain(vec!["--cfg".into(), r#"feature="cargo-clippy""#.into()])
            .collect::<Vec<String>>();

        // We enable Clippy if one of the following conditions is met
        // - IF Clippy is run on its test suite OR
        // - IF Clippy is run on the main crate, not on deps (`!cap_lints_allow`) THEN
        //    - IF `--no-deps` is not set (`!no_deps`) OR
        //    - IF `--no-deps` is set and Clippy is run on the specified primary package
        let cap_lints_allow = arg_value(&orig_args, "--cap-lints", |val| val == "allow").is_some()
            && arg_value(&orig_args, "--force-warn", |val| val.contains("clippy::")).is_none();
        let in_primary_package = env::var("CARGO_PRIMARY_PACKAGE").is_ok();

        let clippy_enabled = !cap_lints_allow && (!no_deps || in_primary_package);
        if clippy_enabled {
            args.extend(clippy_args);
            rustc_driver::RunCompiler::new(&args, &mut ClippyCallbacks { clippy_args_var }).run()
        } else {
            rustc_driver::RunCompiler::new(&args, &mut RustcCallbacks { clippy_args_var }).run()
        }
    }))
}
