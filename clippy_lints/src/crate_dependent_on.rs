use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_no_core_crate, is_no_std_crate};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_session::cstore::ExternCrate;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

// TODO: The example
declare_clippy_lint! {
    /// ### What it does
    /// Checks for dependencies which depend on `std` in `#![no_std]` crates.
    ///
    /// ### Why is this bad?
    /// The dependency will pull in `std` regardless, defeating the whole point of `#![no_std]`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #![no_std]
    ///
    /// // this is unlikely to be what you want!
    /// use i_depend_on_std::*;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// #![no_std]
    ///
    /// // same functionality but without std
    /// use i_dont_depend_on_std::*;
    /// ```
    #[clippy::version = "1.71.0"]
    pub NO_STD_CRATE_STD_DEPENDENT,
    nursery,
    "disallow depending on std in `#![no_std]` crates"
}

// TODO: Tests and the example
declare_clippy_lint! {
    /// ### What it does
    /// Checks for dependencies which pull in `core` in `#![no_core]` crates.
    ///
    /// ### Why is this bad?
    /// The dependency will pull in core regardless, defeating the whole point of `#![no_core]`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #![no_core]
    ///
    /// // this is unlikely to be what you want!
    /// use i_depend_on_core::*;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// #![no_core]
    ///
    /// // same functionality but without core
    /// use i_dont_depend_on_core::*;
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
        // they will only be present once
        let mut std = Vec::with_capacity(1);
        let mut core = Vec::with_capacity(1);

        if !no_std && !no_core {
            return;
        }

        let tcx = cx.tcx;
        for krate in tcx.crates(()) {
            if let Some(ext) = tcx.extern_crate(krate.as_def_id()) {
                let krate_name = tcx.crate_name(*krate);
                if krate_name == sym::std {
                    std.push(*ext);
                } else if krate_name == sym::core {
                    core.push(*ext);
                }
            }
        }

        if no_std {
            check_dependent(cx, NO_STD_CRATE_STD_DEPENDENT, &std, "#![no_std]");
        }
        if no_core {
            check_dependent(cx, NO_CORE_CRATE_CORE_DEPENDENT, &core, "#![no_core]");
        }
    }
}

fn check_dependent(cx: &LateContext<'_>, lint: &'static Lint, exts: &Vec<ExternCrate>, attr: &'static str) {
    for ext in exts {
        span_lint_and_then(
            cx,
            lint,
            ext.span,
            &format!("`{attr}` crate depends on a crate that isn't `{attr}`"),
            |diag| {
                diag.help("find and remove the offending dependency");
                diag.span_help(ext.span, "try removing this dependency");
            },
        );
    }
}
