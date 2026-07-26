use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_in_cfg_test;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{HirId, Path};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::kw;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for paths written using `super`, `self,` or module names declared in the current module, instead
    /// requiring that all imports are absolute via `crate`.
    ///
    /// Does not apply to `super::*` in test modules.
    ///
    /// ### Why restrict this?
    ///
    /// Enforces that items in the local crate are imported using a consistent style (always prefixed by `crate`).
    /// Formatting tools can utilize this style to keep standard library, external, and local imports in separate
    /// groups.
    ///
    /// ### Example
    /// ```no_run
    /// pub mod foo {
    ///     pub struct Bar;
    /// }
    ///
    /// use foo::Bar;
    /// # fn main() {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// pub mod foo {
    ///     pub struct Bar;
    /// }
    ///
    /// use crate::foo::Bar;
    /// # fn main() {}
    /// ```
    #[clippy::version = "1.99.0"]
    pub RELATIVE_PATHS,
    restriction,
    "checks for usage of relative module paths"
}

declare_lint_pass!(RelativePaths => [RELATIVE_PATHS]);

impl LateLintPass<'_> for RelativePaths {
    fn check_path(&mut self, cx: &LateContext<'_>, path: &Path<'_>, hir_id: HirId) {
        let Res::Def(kind, def_id) = path.res else { return };

        // Paths outside of the current crate are always absolute.
        if !def_id.is_local() {
            return;
        }

        // Only lint inside imports.
        if let DefKind::Use = cx.tcx.def_kind(hir_id.owner) {
            // Avoid double-reporting of struct constructor imports.
            if let DefKind::Ctor(..) = kind {
                return;
            }

            let is_relative = path.segments.first().is_some_and(|segment| {
                let is_self = segment.ident.name == kw::SelfLower;
                let is_super = segment.ident.name == kw::Super && !is_in_cfg_test(cx.tcx, hir_id);

                let is_local_mod = if let Some(segment_def_id) = segment.res.opt_def_id() {
                    segment_def_id.is_local() && !segment_def_id.is_crate_root()
                } else {
                    false
                };

                is_self || is_super || is_local_mod
            });

            if is_relative && cx.tcx.visibility(def_id).is_public() {
                let suggestion = format!("crate::{}", cx.tcx.def_path_str(def_id));

                span_lint_and_sugg(
                    cx,
                    RELATIVE_PATHS,
                    path.span,
                    "relative paths are restricted",
                    "use an absolute path",
                    suggestion,
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
