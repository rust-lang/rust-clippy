use clippy_config::Conf;
use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::paths::PathNS;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{ItemKind, OwnerNode, PathSegment, UseKind, UseTree};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured modules in clippy.toml.
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// modules are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some modules are undesirable in certain contexts.
    ///
    /// ### Example:
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// disallowed-modules = [
    ///     # Can use a string as the path of the disallowed type.
    ///     "std::sync",
    ///     # Can also use an inline table with a `path` key.
    ///     { path = "std::net" },
    ///     # When using an inline table, can add a `reason` for why the type
    ///     # is disallowed.
    ///     { path = "std::sync", reason = "no std sync allowed" },
    ///     # Can also add a `replacement` that will be offered as a suggestion.
    ///     { path = "std::sync", reason = "prefer crate internal sync types", replacement = "crate::sync" },
    ///     # This would normally error if the path is incorrect, but with `allow-invalid` = `true`,
    ///     # it will be silently ignored
    ///     { path = "std::invalid::module", reason = "use alternative instead", allow-invalid = true }
    /// ]
    /// ```
    ///
    /// ```rust,ignore
    /// use std::sync;
    /// // or its use
    /// let x = std::sync::Mutext::default();
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // A similar module that is allowed by the config
    /// use crate::sync::Mutex;
    /// ```
    #[clippy::version = "1.98.0"]
    pub DISALLOWED_MODULES,
    style,
    "use of disallowed modules"
}

impl_lint_pass!(DisallowedModules => [DISALLOWED_MODULES]);

pub struct DisallowedModules {
    def_ids: DefIdMap<(&'static str, &'static DisallowedPath)>,
}

impl DisallowedModules {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (def_ids, _) = create_disallowed_map(
            tcx,
            &conf.disallowed_modules,
            PathNS::Type,
            def_kind_predicate,
            "module",
            false,
        );
        Self { def_ids }
    }

    fn check_did(&self, cx: &LateContext<'_>, did: DefId, span: Span) -> Option<()> {
        if let Some(&(path_str, disallowed_path)) = self.def_ids.get(&did) {
            span_lint_and_then(
                cx,
                DISALLOWED_MODULES,
                span,
                format!("use of a disallowed module `{path_str}`"),
                disallowed_path.diag_amendment(span),
            );
            None
        } else {
            Some(())
        }
    }

    fn check_path_segments<'a, 'b>(
        &self,
        cx: &LateContext<'a>,
        segments: impl Iterator<Item = &'b PathSegment<'b>>,
        span: Span,
    ) -> Option<()> {
        for segment in segments {
            if let Res::Def(def_kind, did) = segment.res
                && def_kind_predicate(def_kind)
            {
                self.check_did(cx, did, span)?;
            }
        }
        Some(())
    }

    fn check_res_emit(&self, cx: &LateContext<'_>, res: &Res, span: Span) -> Option<()> {
        if let Some(did) = res.opt_def_id() {
            self.check_did(cx, did, span)?;
        }
        Some(())
    }

    fn check_use_tree(&self, cx: &LateContext<'_>, tree: &UseTree<'_>) {
        let segments_fine = self
            .check_path_segments(cx, tree.prefix.segments.iter(), tree.prefix.span)
            .is_some();

        match tree.kind {
            UseKind::Nested { items } => {
                for (child, ..) in items {
                    self.check_use_tree(cx, child);
                }
            },
            UseKind::Single(_) | UseKind::Glob if segments_fine && let Some(res) = tree.prefix.res.type_ns => {
                self.check_res_emit(cx, &res, tree.prefix.span);
            },
            _ => {},
        }
    }
}

pub fn def_kind_predicate(def_kind: DefKind) -> bool {
    matches!(def_kind, DefKind::Mod)
}

impl<'tcx> LateLintPass<'tcx> for DisallowedModules {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'tcx>) {
        if let ItemKind::Use(tree, ..) = item.kind {
            self.check_use_tree(cx, &tree);
        }
    }

    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &rustc_hir::Path<'tcx>, hid: rustc_hir::HirId) {
        // use items should be checked by check_item in order to deduplicate emits.
        // We also have to check the final res of the use item potentially.
        if let OwnerNode::Item(item) = cx.tcx.hir_owner_node(hid.owner)
            && let ItemKind::Use(..) = item.kind
        {
            return;
        }
        self.check_path_segments(cx, path.segments.iter(), path.span);
    }
}
