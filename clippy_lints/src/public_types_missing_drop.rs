use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::ty::is_copy;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for exported types that have no drop glue, i.e. that neither implement `Drop`
    /// nor contain any field that does.
    ///
    /// ### Why is this bad?
    /// Adding `Drop` later is a breaking change because it alters the lifetime rules
    /// applying to values of a type.  This also happens automatically if any
    /// type this type contains begins to implement `Drop` (due to "drop glue"). This
    /// makes it very challenging to keep an internal type private, or keep a type
    /// from a dependency private.
    ///
    /// Note that excessive `impl Drop` comes with costs (stricter move/borrow rules,
    /// slightly worse codegen, removal of partial moves, etc.)
    ///
    /// ### Known problems
    /// - Does not work for generic types.
    ///
    /// ### Example
    /// ```no_run
    /// # struct PrivateType;
    /// pub struct PublicType(PrivateType);
    /// ```
    /// This is bad because `PrivateType` might gain drop glue in the future (for example by
    /// implementing `Drop`), which would be a breaking change on `PublicType`.  Note that if
    /// `PublicType` *already* has drop glue, this lint will not fire.
    ///
    /// Use instead:
    /// ```no_run
    /// # struct PrivateType;
    /// pub struct PublicType(PrivateType);
    ///
    /// impl Drop for PublicType {
    ///     fn drop(&mut self) {}
    /// }
    /// ```
    #[clippy::version = "1.98.0"]
    pub PUBLIC_TYPES_MISSING_DROP,
    nursery,
    "detects exported types that have no drop glue"
}

declare_lint_pass!(PublicTypesMissingDrop => [PUBLIC_TYPES_MISSING_DROP]);

impl<'tcx> LateLintPass<'tcx> for PublicTypesMissingDrop {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &Item<'tcx>) {
        if !cx.effective_visibilities.is_exported(item.owner_id.def_id) {
            return;
        }

        let (ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::Union(..)) = item.kind else {
            return;
        };

        let ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip();

        if !ty.needs_drop(cx.tcx, cx.typing_env()) && !is_copy(cx, ty) {
            span_lint_and_help(
                cx,
                PUBLIC_TYPES_MISSING_DROP,
                item.span,
                "this exported type has no drop glue",
                None,
                "add an empty `Drop` implementation so that gaining drop glue later is not a breaking change",
            );
        }
    }
}
