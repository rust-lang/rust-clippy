use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_entrypoint_fn;
use clippy_utils::source::snippet_indent;
use rustc_errors::Applicability;
use rustc_hir::{HirId, Impl, ImplItemKind, Item, ItemKind, TraitItemKind, find_attr};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

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

pub fn check_item(cx: &LateContext<'_>, item_hir_id: HirId, span: Span) {
    let attrs = cx.tcx.hir_attrs(item_hir_id);
    if find_attr!(attrs, MustUse { .. }) {
        return;
    }
    let indent = snippet_indent(cx, span).unwrap_or_default();
    span_lint_and_sugg(
        cx,
        MISSING_MUST_USE,
        span.shrink_to_lo(),
        "missing `#[must_use]` attribute on this type",
        "add #[must_use] to this type definition",
        format!("#[must_use]\n{indent}"),
        Applicability::MachineApplicable,
    );
}

impl LateLintPass<'_> for MissingMustUse {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if item.span.in_external_macro(cx.sess().source_map()) {
            return;
        }
        match item.kind {
            ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..) => check_item(cx, item.hir_id(), item.span),
            ItemKind::Fn { .. } => {
                // Skip entry point functions
                if is_entrypoint_fn(cx, item.owner_id.def_id.to_def_id()) {
                    return;
                }
                check_item(cx, item.hir_id(), item.span);
            },
            ItemKind::Trait { items, .. } => {
                for item_ref in items {
                    let item = cx.tcx.hir_trait_item(*item_ref);
                    // Lint functions only
                    if !matches!(item.kind, TraitItemKind::Fn(..)) {
                        continue;
                    }
                    check_item(cx, item.hir_id(), item.span);
                }
            },
            ItemKind::Impl(Impl { items, .. }) => {
                for item_ref in items {
                    let item = cx.tcx.hir_impl_item(*item_ref);
                    // Lint functions only
                    if !matches!(item.kind, ImplItemKind::Fn(..)) {
                        continue;
                    }
                    check_item(cx, item.hir_id(), item.span);
                }
            },
            ItemKind::Const(..)
            | ItemKind::Static(..)
            | ItemKind::Mod(..)
            | ItemKind::Use(..)
            | ItemKind::ForeignMod { .. }
            | ItemKind::GlobalAsm { .. }
            | ItemKind::TyAlias(..)
            | ItemKind::TraitAlias(..)
            | ItemKind::Macro(..)
            | ItemKind::ExternCrate(..) => {},
        }
    }
}
