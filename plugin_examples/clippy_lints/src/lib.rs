#![feature(rustc_private)]

extern crate rustc_lint;
extern crate rustc_session;

use clippy_utils::conf::Conf;
use rustc_session::Session;

#[no_mangle]
pub extern "C" fn register_plugins(store: &mut rustc_lint::LintStore, sess: &Session, conf: &Conf) {
    clippy_lints::register_plugins(store, sess, conf);
}
