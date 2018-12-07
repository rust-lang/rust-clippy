// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use crate::rustc::{declare_tool_lint, lint_array};
use crate::syntax::ast;
use crate::utils::span_lint;

/// **What it does:** Checks for the `get_` prefix on getters.
///
/// **Why is this bad?** The Rust API Guidelines section on naming
/// [specifies](https://rust-lang-nursery.github.io/api-guidelines/naming.html#getter-names-follow-rust-convention-c-getter)
/// that the `get_` prefix is not used for getters in Rust code unless
/// there is a single and obvious thing that could reasonably be gotten by
/// a getter.
///
/// The exceptions to this naming convention are as follows:
/// - `get` (such as in
///   [`std::cell::Cell::get`](https://doc.rust-lang.org/std/cell/struct.Cell.html#method.get))
/// - `get_mut`
/// - `get_unchecked`
/// - `get_unchecked_mut`
/// - `get_ref`
///
/// **Known problems:** None.
///
/// **Example:**
///
/// ```rust
/// // Bad
/// impl B {
///     fn get_id(&self) -> usize {
///         ..
///     }
/// }
///
/// // Good
/// impl G {
///     fn id(&self) -> usize {
///         ..
///     }
/// }
///
/// // Also allowed
/// impl A {
///     fn get(&self) -> usize {
///         ..
///     }
/// }
/// ```
declare_clippy_lint! {
    pub GETTER_PREFIX,
    style,
    "prefixing a getter with `get_`, which does not follow convention"
}

#[derive(Copy, Clone)]
pub struct GetterPrefix;

#[rustfmt::skip]
const ALLOWED_METHOD_NAMES: [&'static str; 5] = [
    "get",
    "get_mut",
    "get_unchecked",
    "get_unchecked_mut",
    "get_ref"
];

impl LintPass for GetterPrefix {
    fn get_lints(&self) -> LintArray {
        lint_array!(GETTER_PREFIX)
    }
}

impl EarlyLintPass for GetterPrefix {
    fn check_impl_item(&mut self, cx: &EarlyContext<'_>, implitem: &ast::ImplItem) {
        if let ast::ImplItemKind::Method(..) = implitem.node {
            let name = implitem.ident.name.as_str().get();
            if name.starts_with("get_") && !ALLOWED_METHOD_NAMES.contains(&name) {
                span_lint(
                    cx,
                    GETTER_PREFIX,
                    implitem.span,
                    "prefixing a getter with `get_` does not follow naming conventions",
                );
            }
        }
    }
}
