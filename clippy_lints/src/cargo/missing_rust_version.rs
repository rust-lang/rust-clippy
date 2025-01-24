use cargo_metadata::Metadata;
use clippy_utils::diagnostics::span_lint;
use clippy_utils::msrvs::{self, Msrv};
use rustc_lint::LateContext;
use rustc_span::DUMMY_SP;

use super::MISSING_RUST_VERSION;

pub(super) fn check(cx: &LateContext<'_>, metadata: &Metadata, msrv: &Msrv) {
    // see <https://github.com/oli-obk/cargo_metadata/issues/253>
    if !msrv.meets(msrvs::RUST_VERSION_FIELD) {
        return;
    }

    for package in &metadata.packages {
        if package.rust_version.is_none() {
            let message = format!("package `{}` is missing the `rust-version` field", package.name);
            span_lint(cx, MISSING_RUST_VERSION, DUMMY_SP, message);
        }
    }
}
