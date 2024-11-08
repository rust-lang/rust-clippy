use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects types with `Unsafe` in the name that are publically constructable.
    ///
    /// ### Why is this bad?
    ///
    /// `Unsafe` in the name of a type implies that there is some kind of safety invariant
    /// being held by constructing said type, however, this invariant may not be checked
    /// if a user can safely publically construct it.
    ///
    /// ### Example
    /// ```no_run
    /// pub struct UnsafeToken {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// pub struct UnsafeToken {
    ///     _private: ()
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub CONSTRUCTABLE_UNSAFE_TYPE,
    suspicious,
    "`Unsafe` types that are publically constructable"
}

declare_lint_pass!(ConstructableUnsafeType => [CONSTRUCTABLE_UNSAFE_TYPE]);

impl LateLintPass<'_> for ConstructableUnsafeType {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Struct(variant, generics) = item.kind
            && {
                // If the type contains `Unsafe`, but is not exactly.
                let name = item.ident.as_str();
                name.contains("Unsafe") && name.len() != "Unsafe".len()
            }
            && generics.params.is_empty()
            && cx.effective_visibilities.is_reachable(item.owner_id.def_id)
            && variant
                .fields()
                .iter()
                .all(|f| cx.effective_visibilities.is_exported(f.def_id))
        {
            span_lint_and_help(
                cx,
                CONSTRUCTABLE_UNSAFE_TYPE,
                item.span,
                "`Unsafe` type is publically constructable",
                None,
                "give this type a private field, or make it private",
            );
        }
    }
}
