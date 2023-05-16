use clippy_utils::diagnostics::span_lint;
use clippy_utils::{diagnostics::span_lint_and_help, is_no_core_crate, is_no_std_crate};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_session::cstore::ExternCrateSource;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::DUMMY_SP;
use rustc_span::{sym, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for dependencies which depend on `std` in `#![no_std]` crates.
    ///
    /// ### Why is this bad?
    /// The dependency will pull in std regardless, defeating the whole point of `#![no_std]`.
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
    "default lint description"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for dependencies which depend on `core` in `#![no_core]` crates.
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
    "default lint description"
}

declare_lint_pass!(CrateDependentOn => [NO_STD_CRATE_STD_DEPENDENT, NO_CORE_CRATE_CORE_DEPENDENT]);

impl LateLintPass<'_> for CrateDependentOn {
    fn check_crate(&mut self, cx: &LateContext<'_>) {
        // TODO: This entire thing is TODO. This method of finding std/core has a lot of flaws, as
        // it cannot find multiple dependencies at once and cannot find the true name of the
        // dependency if it's not depended on by the crate directly

        let mut std = vec![];
        let mut core = vec![];

        let tcx = cx.tcx;
        for krate in tcx.crates(()) {
            let krate_name = tcx.crate_name(*krate);

            if let Some(ext) = tcx.extern_crate(krate.as_def_id()) {
                let dep_name = tcx.crate_name(ext.dependency_of);

                if krate_name == sym::std {
                    std.push((ext, ext.span));
                } else if krate_name == sym::core {
                    core.push((ext, ext.span));
                }
            }
        }

        for (krate, span) in std {
            span_lint(
                cx,
                NO_STD_CRATE_STD_DEPENDENT,
                span,
                &format!(
                    "lol {krate:#?} {}",
                    if let ExternCrateSource::Extern(def_id) = krate.src {
                        format!("{:?}", tcx.crate_name(def_id.krate))
                    } else {
                        format!("no")
                    }
                ),
            );
        }
    }
}

fn report_dependent_crate(
    cx: &LateContext<'_>,
    span: Option<Span>,
    lint: &'static Lint,
    attr: &'static str,
    dep_name: Symbol,
) {
    span_lint_and_help(
        cx,
        lint,
        span.unwrap_or(DUMMY_SP),
        &format!("`{attr}` crate depends on a crate that isn't `{attr}`: `{}`", dep_name),
        None,
        "remove the offending dependency",
    );
}

fn report_crate(cx: &LateContext<'_>, lint: &'static Lint, attr: &'static str) {
    span_lint_and_help(
        cx,
        lint,
        DUMMY_SP,
        &format!("`{attr}` crate depends on a crate that isn't `{attr}`"),
        None,
        "find and remove the offending dependency",
    );
}
