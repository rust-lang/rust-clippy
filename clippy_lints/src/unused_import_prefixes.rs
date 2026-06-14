use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::res::MaybeDef;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{HirId, Item, ItemKind, Mod};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, kw};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for redundant `crate::...` prefixes in imports (use statements)
    ///
    /// ### Why is this bad?
    ///
    /// Unnecessarily increases noise and complicates imports.
    ///
    /// ### Example
    /// ```no_run
    /// // src/foo.rs
    /// pub mod bar {}
    ///     pub struct Baz;
    /// }
    ///
    /// use crate::foo::bar::Baz;
    /// ```
    /// Use instead:
    /// ```no_run
    /// // src/foo.rs
    /// pub mod bar {}
    ///     pub struct Baz;
    /// }
    ///
    /// use self::bar::Baz;
    /// ```
    #[clippy::version = "1.98.0"]
    pub UNUSED_IMPORT_PREFIXES,
    nursery,
    "redundant `crate::...` prefix in an use statement"
}

impl_lint_pass!(UnusedImportPrefixes => [UNUSED_IMPORT_PREFIXES]);

#[derive(Default)]
pub struct UnusedImportPrefixes {
    processed: FxHashSet<Span>,
}

impl LateLintPass<'_> for UnusedImportPrefixes {
    fn check_mod(&mut self, _: &LateContext<'_>, _: &Mod<'_>, _: HirId) {
        self.processed.clear();
    }

    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        let ItemKind::Use(use_path, _) = item.kind else {
            return;
        };

        // Ignore use statements expanded from macros
        if item.span.from_expansion() {
            return;
        }

        // Only check imports starting with the `crate` keyword
        if use_path.segments.is_empty() || use_path.segments[0].ident.name != kw::Crate {
            return;
        }

        // Prevent duplicate lints for the same use statement
        let root_span = use_path.segments[0].ident.span;
        if !self.processed.insert(root_span) {
            return;
        }

        let module_def_id = cx.tcx.parent_module(item.hir_id()).to_def_id();
        let Some(target_def_id) = use_path.res.iter().find_map(|res| res.opt_def_id()) else {
            return;
        };

        // Checks if the use statement is really redundant
        if !cx.tcx.is_descendant_of(target_def_id, module_def_id) && target_def_id != module_def_id {
            return;
        }

        let mut redundant_len = 0;
        for (i, mod_sym) in cx
            .tcx
            .def_path(module_def_id)
            .data
            .iter()
            .filter_map(|s| s.data.get_opt_name())
            .enumerate()
        {
            redundant_len += 1;
            let Some(segment) = use_path.segments.get(i + 1) else {
                return;
            };
            if mod_sym != segment.ident.name {
                return;
            }
        }

        let prefix_span = use_path.segments[0]
            .ident
            .span
            .to(use_path.segments[redundant_len].ident.span);

        let snippet = clippy_utils::source::snippet_opt(cx, prefix_span).unwrap_or_default();

        if snippet.contains(['{', '}']) {
            span_lint_and_help(
                cx,
                UNUSED_IMPORT_PREFIXES,
                prefix_span,
                "redundant `crate::...` prefix in a use statement",
                None,
                "consider removing the redundant prefix manually",
            );
        } else {
            span_lint_and_sugg(
                cx,
                UNUSED_IMPORT_PREFIXES,
                prefix_span,
                UNUSED_IMPORT_PREFIXES.desc,
                "remove redundant use path prefix",
                "self".to_string(),
                Applicability::MachineApplicable,
            );
        }
    }
}
