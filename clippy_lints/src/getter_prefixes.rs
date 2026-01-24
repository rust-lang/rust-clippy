use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::ast::*;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// **What it does:** Checks for the `get_` prefix on getters.
    ///
    /// **Why is this bad?** The Rust API Guidelines section on naming
    /// [specifies](https://rust-lang-nursery.github.io/api-guidelines/naming.html#getter-names-follow-rust-convention-c-getter)
    /// that the `get_` prefix is not used for getters in Rust code unless
    /// there is a single and obvious thing that could reasonably be gotten by
    /// a getter.
    ///
    /// The exceptions to this naming convention are as follows:
    /// - `get` (such as in
    ///   [`std::cell::Cell::get`](https://doc.rust-lang.org/std/cell/struct.Cell.html#method.get))
    /// - `get_mut`
    /// - `get_unchecked`
    /// - `get_unchecked_mut`
    /// - `get_ref`
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // Bad
    /// impl B {
    ///     fn get_id(&self) -> usize {
    ///         ..
    ///     }
    /// }
    ///
    /// // Good
    /// impl G {
    ///     fn id(&self) -> usize {
    ///         ..
    ///     }
    /// }
    ///
    /// // Also allowed
    /// impl A {
    ///     fn get(&self) -> usize {
    ///         ..
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.95.0"]
    pub GETTER_PREFIXES,
    style,
    "prefixing a getter with `get_`, which does not follow convention"
}

const EXCLUDED_SUFFIXES: [&str; 5] = ["", "mut", "unchecked", "unchecked_mut", "ref"];

#[derive(Default)]
pub struct GetterPrefixes {
    inherent_impl_ctx: bool,
}

impl_lint_pass!(GetterPrefixes => [GETTER_PREFIXES]);

impl EarlyLintPass for GetterPrefixes {
    fn check_item(&mut self, _: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Impl(Impl { of_trait: None, .. }) = item.kind {
            self.inherent_impl_ctx = true
        }
    }

    fn check_item_post(&mut self, _: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Impl(_) = item.kind {
            self.inherent_impl_ctx = false
        }
    }

    fn check_trait_item(&mut self, cx: &EarlyContext<'_>, assoc_item: &AssocItem) {
        check_getter_prefix(cx, assoc_item);
    }

    fn check_impl_item(&mut self, cx: &EarlyContext<'_>, assoc_item: &AssocItem) {
        if self.inherent_impl_ctx {
            check_getter_prefix(cx, assoc_item);
        }
    }
}

fn check_getter_prefix(cx: &EarlyContext<'_>, assoc_item: &AssocItem) {
    if let AssocItemKind::Fn(box Fn {
        ref ident,
        sig: FnSig { ref decl, .. },
        ..
    }) = assoc_item.kind
        && decl.has_self()
        && let FnRetTy::Ty(box Ty { ref kind, .. }) = decl.output
        && !kind.is_unit()
        && let Some(ref suffix) = ident.name.as_str().strip_prefix("get_")
        && !EXCLUDED_SUFFIXES.contains(suffix)
    {
        span_lint_and_sugg(
            cx,
            GETTER_PREFIXES,
            ident.span,
            "prefixing a getter with `get_` does not follow naming conventions",
            "replace it with",
            suffix.to_string(),
            Applicability::Unspecified,
        );
    }
}
