use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_in_cfg_test;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, PerNS, Res};
use rustc_hir::{HirId, Item, ItemKind, UseKind, UseTree};
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
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Use(use_tree) = &item.kind {
            check_prefix(cx, use_tree, item.hir_id());
        }
    }
}

fn check_prefix(cx: &LateContext<'_>, use_tree: &UseTree<'_>, item_id: HirId) {
    let Some(segment) = use_tree.prefix.segments.first() else {
        return;
    };

    let is_self = segment.ident.name == kw::SelfLower;
    let is_super = segment.ident.name == kw::Super && !is_in_cfg_test(cx.tcx, item_id);

    let is_local_mod = if let Some(segment_def_id) = segment.res.opt_def_id() {
        segment_def_id.is_local() && !segment_def_id.is_crate_root()
    } else {
        false
    };

    let is_relative = is_self || is_super || is_local_mod;

    let any_public_defs = use_tree
        .resolutions()
        .flat_map(PerNS::present_items)
        .filter_map(|res| {
            let Res::Def(kind, def_id) = res else {
                return None;
            };

            // Avoid double-reporting of struct constructor imports.
            if let DefKind::Ctor(..) = kind {
                return None;
            }

            Some(def_id)
        })
        .any(|def_id| cx.tcx.visibility(def_id).is_public());

    if is_relative && any_public_defs {
        let res = match use_tree.kind {
            UseKind::Single(..) => use_tree.prefix.res.present_items().next().unwrap(),
            _ => use_tree.prefix.segments.last().unwrap().res,
        };

        let Res::Def(_, prefix_def_id) = res else {
            return;
        };

        let path_str = cx.tcx.def_path_str(prefix_def_id);
        let suggestion = if prefix_def_id.is_local() {
            format!("crate::{path_str}")
        } else {
            path_str
        };

        span_lint_and_sugg(
            cx,
            RELATIVE_PATHS,
            use_tree.prefix.span,
            "relative paths are restricted",
            "use an absolute path",
            suggestion,
            Applicability::MachineApplicable,
        );
    }
}
