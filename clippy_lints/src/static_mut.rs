use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Item, ItemKind, Mutability, StaticItem};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Produces warnings when a `static mut` is declared.
    ///
    /// ### Why is this bad?
    /// `static mut` can [easily produce undefined behavior][1] and
    /// [may be removed in the future][2].
    ///
    /// ### Example
    /// ```no_run
    /// static mut GLOBAL_INT: u8 = 0;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::sync::RwLock;
    ///
    /// static GLOBAL_INT: RwLock<u8> = RwLock::new(0);
    /// ```
    ///
    /// [1]: https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-reference.html
    /// [2]: https://github.com/rust-lang/rfcs/pull/3560
    #[clippy::version = "1.80.0"]
    pub STATIC_MUT,
    nursery,
    "detect mutable static definitions"
}

declare_lint_pass!(StaticMut => [STATIC_MUT]);

impl EarlyLintPass for StaticMut {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if in_external_macro(cx.sess(), item.span) {
            return;
        };
        let ItemKind::Static(ref static_item_box) = item.kind else {
            return;
        };
        let StaticItem {
            mutability: Mutability::Mut,
            ..
        } = static_item_box.as_ref()
        else {
            return;
        };
        span_lint_and_help(
            cx,
            STATIC_MUT,
            item.span,
            "declaration of static mut",
            None,
            "remove the `mut` and use a type with interior mutibability that implements `Sync`, such as `std::sync::Mutex`",
        );
    }
}
