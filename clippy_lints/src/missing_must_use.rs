use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_indent;
use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind, find_attr};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// It finds types that are not marked with `#[must_use]`.
    ///
    /// ### Why restrict this?
    /// Marking a type with `#[must_use]` ensures that the type cannot be silently discarded.
    /// This is especially important for types that represent resources, handles, or results,
    /// where ignoring the value is almost certainly a bug.
    ///
    /// Enabling this lint enforces that every type definition is explicitly considered for
    /// `#[must_use]` annotation, rather than relying on authors to remember to add it.
    ///
    /// Types that genuinely do not need the attribute can be `#[allow]`ed individually with
    /// a justifying comment.
    ///
    /// ### Example
    /// ```no_run
    /// struct S(u8);   // missing `#[must_use]` and the suggestion to add `#[must_use]` will be triggered.
    /// ```
    #[clippy::version = "1.97.0"]
    pub MISSING_MUST_USE,
    restriction,
    "finding types that are not marked with `#[must_use]`"
}

declare_lint_pass!(MissingMustUse => [MISSING_MUST_USE]);

impl LateLintPass<'_> for MissingMustUse {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if item.span.in_external_macro(cx.sess().source_map()) {
            return;
        }
        let attrs = cx.tcx.hir_attrs(item.hir_id());
        match item.kind {
            ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..) => {
                if !find_attr!(attrs, MustUse { .. }) {
                    let indent = snippet_indent(cx, item.span).unwrap_or_default();
                    span_lint_and_sugg(
                        cx,
                        MISSING_MUST_USE,
                        item.span.shrink_to_lo(),
                        "missing `#[must_use]` attribute on this type",
                        "add #[must_use] to this type definition",
                        format!("#[must_use]\n{indent}"),
                        Applicability::MachineApplicable,
                    );
                }
            },
            ItemKind::Const(..)
            | ItemKind::Static(..)
            | ItemKind::Fn { .. }
            | ItemKind::Mod(..)
            | ItemKind::Use(..)
            | ItemKind::ForeignMod { .. }
            | ItemKind::GlobalAsm { .. }
            | ItemKind::TyAlias(..)
            | ItemKind::Trait(..)
            | ItemKind::Impl { .. }
            | ItemKind::TraitAlias(..)
            | ItemKind::Macro(..)
            | ItemKind::ExternCrate(..) => {},
        }
    }
}
