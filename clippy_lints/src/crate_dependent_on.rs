use clippy_utils::diagnostics::span_lint;
use clippy_utils::{is_no_core_crate, is_no_std_crate};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_session::cstore::ExternCrate;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks if a `#![no_std]` crate depends on `std`.
    ///
    /// ### Why is this bad?
    /// It will pull in `std` regardless, defeating the whole point of `#![no_std]`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #![no_std]
    ///
    /// // this is unlikely to be what you want!
    /// use i_depend_on_std::*;
    /// ```
    /// ```
    #[clippy::version = "1.71.0"]
    pub NO_STD_CRATE_STD_DEPENDENT,
    nursery,
    "disallow depending on std in `#![no_std]` crates"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks if a `#![no_core]` crate depends on `core`.
    ///
    /// ### Why is this bad?
    /// It will pull in `std` regardless, defeating the whole point of `#![no_core]`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #![no_core]
    ///
    /// // this is unlikely to be what you want!
    /// use i_depend_on_core::*;
    /// ```
    #[clippy::version = "1.71.0"]
    pub NO_CORE_CRATE_CORE_DEPENDENT,
    nursery,
    "disallow depending on core in `#![no_core]` crates"
}

declare_lint_pass!(CrateDependentOn => [NO_STD_CRATE_STD_DEPENDENT, NO_CORE_CRATE_CORE_DEPENDENT]);

impl LateLintPass<'_> for CrateDependentOn {
    fn check_crate(&mut self, cx: &LateContext<'_>) {
        let (no_std, no_core) = (is_no_std_crate(cx), is_no_core_crate(cx));
        // afaik, an extern crate will only appear once in `tcx.crates` so we don't need to handle a case
        // where there are multiple of them. This unfortunately also means we cannot reliably trace it back
        // to find the dependency that the original crate depends on, that inadvertently depends on
        // std and/or core.
        let mut std = None;
        let mut core = None;

        if !no_std && !no_core {
            return;
        }

        let tcx = cx.tcx;
        for krate in tcx.crates(()) {
            if let Some(ext) = tcx.extern_crate(krate.as_def_id()) {
                let krate_name = tcx.crate_name(*krate);
                if krate_name == sym::std {
                    std.get_or_insert(ext);
                } else if krate_name == sym::core {
                    core.get_or_insert(ext);
                }
            }
        }

        check_dependent(cx, NO_STD_CRATE_STD_DEPENDENT, std, "#![no_std]", no_std);
        check_dependent(cx, NO_CORE_CRATE_CORE_DEPENDENT, core, "#![no_core]", no_core);
    }
}

fn check_dependent(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    ext: Option<&ExternCrate>,
    attr: &'static str,
    attr_set: bool,
) {
    if attr_set && let Some(ext) = ext {
        span_lint(
            cx,
            lint,
            ext.span,
            &format!("`{attr}` crate depends on a crate that isn't `{attr}`"),
        );
    }
}
