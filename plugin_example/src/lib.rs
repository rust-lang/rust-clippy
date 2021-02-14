#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::conf::Conf;
use rustc_session::Session;

mod allow_clippy_lints;

#[no_mangle]
pub extern "C" fn register_plugins(store: &mut rustc_lint::LintStore, _sess: &Session, _conf: &Conf) {
    store.register_lints(&[&allow_clippy_lints::ALLOW_CLIPPY_LINTS]);
    store.register_late_pass(|| Box::new(allow_clippy_lints::AllowClippyLints));
}
