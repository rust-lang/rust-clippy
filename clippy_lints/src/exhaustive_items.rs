use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::indent_of;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Warns on any exported `enum`s that are not tagged `#[non_exhaustive]`
    ///
    /// ### Why is this bad?
    /// Exhaustive enums are typically fine, but a project which does
    /// not wish to make a stability commitment around exported enums may wish to
    /// disable them by default.
    ///
    /// ### Example
    /// ```rust
    /// enum Foo {
    ///     Bar,
    ///     Baz
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[non_exhaustive]
    /// enum Foo {
    ///     Bar,
    ///     Baz
    /// }
    /// ```
    pub EXHAUSTIVE_ENUMS,
    restriction,
    "detects exported enums that have not been marked #[non_exhaustive]"
}

declare_clippy_lint! {
    /// ### What it does
    /// Warns on any exported `structs`s that are not tagged `#[non_exhaustive]`
    ///
    /// ### Why is this bad?
    /// Exhaustive structs are typically fine, but a project which does
    /// not wish to make a stability commitment around exported structs may wish to
    /// disable them by default.
    ///
    /// ### Example
    /// ```rust
    /// struct Foo {
    ///     bar: u8,
    ///     baz: String,
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[non_exhaustive]
    /// struct Foo {
    ///     bar: u8,
    ///     baz: String,
    /// }
    /// ```
    pub EXHAUSTIVE_STRUCTS,
    restriction,
    "detects exported structs that have not been marked #[non_exhaustive]"
}

declare_lint_pass!(ExhaustiveItems => [EXHAUSTIVE_ENUMS, EXHAUSTIVE_STRUCTS]);

impl LateLintPass<'_> for ExhaustiveItems {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if_chain! {
            if let ItemKind::Enum(..) | ItemKind::Struct(..) = item.kind;
            if cx.access_levels.is_exported(item.hir_id());
            let attrs = cx.tcx.hir().attrs(item.hir_id());
            if !attrs.iter().any(|a| a.has_name(sym::non_exhaustive));
            then {
                let (lint, msg) = if let ItemKind::Struct(ref v, ..) = item.kind {
                    if v.fields().iter().any(|f| !f.vis.node.is_pub()) {
                        // skip structs with private fields
                        return;
                    }
                    (EXHAUSTIVE_STRUCTS, "exported structs should not be exhaustive")
                } else {
                    (EXHAUSTIVE_ENUMS, "exported enums should not be exhaustive")
                };
                let suggestion_span = item.span.shrink_to_lo();
                let indent = " ".repeat(indent_of(cx, item.span).unwrap_or(0));
                span_lint_and_then(
                    cx,
                    lint,
                    item.span,
                    msg,
                    |diag| {
                        let sugg = format!("#[non_exhaustive]\n{}", indent);
                        diag.span_suggestion(suggestion_span,
                                             "try adding #[non_exhaustive]",
                                             sugg,
                                             Applicability::MaybeIncorrect);
                    }
                );

            }
        }
    }
}
