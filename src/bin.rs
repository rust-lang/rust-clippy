#![feature(type_macros)]
#![feature(box_syntax)]
#![feature(rustc_private)]
#![feature(custom_attribute)]
#![feature(slice_patterns)]
#![feature(question_mark)]
#![feature(stmt_expr_attributes)]
#![allow(indexing_slicing, shadow_reuse, unknown_lints, plugin_as_library)]

extern crate rustc;
extern crate rustc_driver;
extern crate rustc_plugin;
extern crate syntax;
extern crate getopts;
extern crate clippy;

use rustc_driver::{driver, CompilerCalls, RustcDefaultCalls, Compilation};
use rustc::session::{config, Session};
use rustc::session::config::{Input, ErrorOutputType};
use syntax::diagnostics;
use std::path::PathBuf;
use std::process::Command;

struct ClippyCompilerCalls(RustcDefaultCalls);

impl std::default::Default for ClippyCompilerCalls {
    fn default() -> Self {
        Self::new()
    }
}

impl ClippyCompilerCalls {
    fn new() -> Self {
        ClippyCompilerCalls(RustcDefaultCalls)
    }
}

impl<'a> CompilerCalls<'a> for ClippyCompilerCalls {
    fn early_callback(&mut self,
                      matches: &getopts::Matches,
                      sopts: &config::Options,
                      descriptions: &diagnostics::registry::Registry,
                      output: ErrorOutputType)
                      -> Compilation {
        self.0.early_callback(matches, sopts, descriptions, output)
    }
    fn no_input(&mut self,
                matches: &getopts::Matches,
                sopts: &config::Options,
                odir: &Option<PathBuf>,
                ofile: &Option<PathBuf>,
                descriptions: &diagnostics::registry::Registry)
                -> Option<(Input, Option<PathBuf>)> {
        self.0.no_input(matches, sopts, odir, ofile, descriptions)
    }
    fn late_callback(&mut self,
                     matches: &getopts::Matches,
                     sess: &Session,
                     input: &Input,
                     odir: &Option<PathBuf>,
                     ofile: &Option<PathBuf>)
                     -> Compilation {
        self.0.late_callback(matches, sess, input, odir, ofile)
    }
    fn build_controller(&mut self, sess: &Session, matches: &getopts::Matches) -> driver::CompileController<'a> {
        let mut control = self.0.build_controller(sess, matches);

        let old = std::mem::replace(&mut control.after_parse.callback, box |_| {});
        control.after_parse.callback = Box::new(move |state| {
            {
                let mut registry = rustc_plugin::registry::Registry::new(state.session, state.krate.as_ref().expect("at this compilation stage the krate must be parsed"));
                registry.args_hidden = Some(Vec::new());
                clippy::plugin_registrar(&mut registry);

                let rustc_plugin::registry::Registry { early_lint_passes, late_lint_passes, lint_groups, llvm_passes, attributes, mir_passes, .. } = registry;
                let sess = &state.session;
                let mut ls = sess.lint_store.borrow_mut();
                for pass in early_lint_passes {
                    ls.register_early_pass(Some(sess), true, pass);
                }
                for pass in late_lint_passes {
                    ls.register_late_pass(Some(sess), true, pass);
                }

                for (name, to) in lint_groups {
                    ls.register_group(Some(sess), true, name, to);
                }

                sess.plugin_llvm_passes.borrow_mut().extend(llvm_passes);
                sess.mir_passes.borrow_mut().extend(mir_passes);
                sess.plugin_attributes.borrow_mut().extend(attributes);
            }
            old(state);
        });

        control
    }
}

use std::path::Path;

pub fn main() {
    use std::env;

    let dep_path = env::current_dir().expect("current dir is not readable").join("target").join("debug").join("deps");

    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    let sys_root = match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("SYSROOT").map(|s| s.to_owned())
                                   .or(Command::new("rustc").arg("--print")
                                                            .arg("sysroot")
                                                            .output().ok()
                                                            .and_then(|out| String::from_utf8(out.stdout).ok())
                                                            .map(|s| s.trim().to_owned())
                                                            )
                .expect("need to specify SYSROOT env var during clippy compilation, or use rustup or multirust"),
    };

    if let Some("clippy") = std::env::args().nth(1).as_ref().map(AsRef::as_ref) {
        let args = wrap_args(std::env::args().skip(2), dep_path, sys_root);
        let path = std::env::current_exe().expect("current executable path invalid");
        let run = std::process::Command::new("cargo")
            .args(&args)
            .env("RUSTC", path)
            .spawn().expect("could not run cargo")
            .wait().expect("failed to wait for cargo?")
            .success();
        assert!(run, "cargo rustc failed");
    } else {
        let args: Vec<String> = if env::args().any(|s| s == "--sysroot") {
            env::args().collect()
        } else {
            env::args().chain(Some("--sysroot".to_owned())).chain(Some(sys_root)).collect()
        };
        rustc_driver::run_compiler(&args, &mut ClippyCompilerCalls::new());
    }
}

fn wrap_args<P, I>(old_args: I, dep_path: P, sysroot: String) -> Vec<String>
    where P: AsRef<Path>, I: Iterator<Item=String> {

    let mut args = vec!["rustc".to_owned()];

    let mut found_dashes = false;
    for arg in old_args {
        found_dashes |= arg == "--";
        args.push(arg);
    }
    if !found_dashes {
        args.push("--".to_owned());
    }
    args.push("-L".to_owned());
    args.push(dep_path.as_ref().to_string_lossy().into_owned());
    args.push(String::from("--sysroot"));
    args.push(sysroot);
    args.push("-Zno-trans".to_owned());
    args
}