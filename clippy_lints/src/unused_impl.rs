use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::DiagExt;
use rustc_ast::ast::{Impl, Item, ItemKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for implementations with an empty body and without a trait.
    ///
    /// ### Why is this bad?
    /// Adds unnecessary clutter.
    ///
    /// ### Example
    /// ```no_run
    /// struct Foo;
    /// impl Foo {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct Foo;
    /// ```
    #[clippy::version = "1.96.0"]
    pub UNUSED_IMPL,
    style,
    "empty implementation, which adds unnecessary clutter"
}

declare_lint_pass!(UnusedImpl => [UNUSED_IMPL]);

impl EarlyLintPass for UnusedImpl {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if item.span.from_expansion() {
            return;
        }

        if let ItemKind::Impl(Impl {
            of_trait: None, items, ..
        }) = &item.kind
            && items.is_empty()
            && item
                .attrs
                .iter()
                .all(|attr| !attr.has_name(sym::cfg) && !attr.is_doc_comment())
        {
            span_lint_and_then(cx, UNUSED_IMPL, item.span, "empty impl body", |diag| {
                diag.suggest_remove_item(
                    cx,
                    item.span_with_attributes(),
                    "remove this",
                    Applicability::MachineApplicable,
                );
            });
        }
    }
}
