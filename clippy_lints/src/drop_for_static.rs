use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Item, ItemKind, MutTy, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `static` variables whose types implement `Drop`.
    ///
    /// ### Why restrict this?
    /// Rust does not call `Drop::drop` for `static` variables at the end of a program's
    /// execution. If a type relies on its `Drop` implementation to release resources
    /// (like closing files, releasing locks, or deleting temporary files), these
    /// actions will never occur for a `static` instance, potentially leading to
    /// resource leaks or inconsistent state.
    ///
    /// ### Example
    /// ```rust
    /// struct Logger;
    ///
    /// impl Drop for Logger {
    ///     fn drop(&mut self) {
    ///         println!("Closing log file...");
    ///     }
    /// }
    ///
    /// static GLOBAL_LOGGER: Logger = Logger;
    /// ```
    ///
    /// ### Known problems
    /// If the type is intended to exist for the lifetime of the program and the
    /// resource is automatically reclaimed by the operating system (like memory),
    /// this lint may be noisy. However, it still serves as a useful reminder that
    /// the `drop` logic will not execute.
    #[clippy::version = "1.93.0"]
    pub DROP_FOR_STATIC,
    nursery,
    "static items with a type that implements 'Drop'"
}
declare_lint_pass!(DropForStatic => [DROP_FOR_STATIC]);

impl LateLintPass<'_> for DropForStatic {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let Some(drop_trait_def_id) = cx.tcx.lang_items().drop_trait()
            && let ItemKind::Static(_, _, Ty { kind, hir_id, .. }, _) = item.kind
        {
            let mut walk_kinds = vec![kind];
            while let Some(kind) = walk_kinds.pop() {
                match kind {
                    TyKind::Path(path) => {
                        let def_id = cx.qpath_res(path, *hir_id).def_id();
                        let ty = cx.tcx.type_of(def_id).instantiate_identity();
                        cx.tcx.for_each_relevant_impl(drop_trait_def_id, ty, |_| {
                            span_lint(cx, DROP_FOR_STATIC, item.span, "static items with drop implementation");
                        });
                    },
                    TyKind::Array(ty, _) | TyKind::Slice(ty) | TyKind::Ref(_, MutTy { ty, .. }) => {
                        walk_kinds.push(&ty.kind);
                    },
                    TyKind::Tup(ty) => walk_kinds.extend(ty.iter().map(|ty| &ty.kind)),
                    _ => {
                    },
                }
            }
        }
    }
}
