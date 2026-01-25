use clippy_utils::diagnostics::span_lint;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{Visitor, walk_path};
use rustc_hir::{HirId, Item, ItemKind, MutTy, Path, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
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
    fn check_item<'a>(&mut self, cx: &LateContext<'a>, item: &Item<'a>) {
        if let Some(drop_trait_def_id) = cx.tcx.lang_items().drop_trait()
            && let ItemKind::Static(_, _, Ty { kind, hir_id, span }, _) = item.kind
        {
            let mut walk_kinds = vec![kind];
            while let Some(kind) = walk_kinds.pop() {
                match kind {
                    TyKind::Path(qpath) => {
                        let mut visitor = PathVisitor::new(cx, drop_trait_def_id);
                        visitor.visit_qpath(&qpath, *hir_id, *span);
                        if visitor.drop_for_static_found {
                            span_lint(cx, DROP_FOR_STATIC, item.span, "static items with drop implementation");
                        }
                    },
                    TyKind::Array(ty, _) | TyKind::Slice(ty) | TyKind::Ref(_, MutTy { ty, .. }) => {
                        walk_kinds.push(&ty.kind);
                    },
                    TyKind::Tup(ty) => walk_kinds.extend(ty.iter().map(|ty| &ty.kind)),
                    _ => {},
                }
            }
        }
    }
}

struct PathVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    drop_trait_def_id: DefId,
    drop_for_static_found: bool,
}
impl<'tcx> PathVisitor<'_, 'tcx> {
    fn new<'a>(cx: &'a LateContext<'tcx>, drop_trait_def_id: DefId) -> PathVisitor<'a, 'tcx> {
        PathVisitor {
            cx,
            drop_trait_def_id,
            drop_for_static_found: false,
        }
    }
}

impl<'tcx> Visitor<'tcx> for PathVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::All;

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) {
        if let Some(def_id) = path.res.opt_def_id() {
            let ty = self.cx.tcx.type_of(def_id).instantiate_identity();
            self.cx.tcx.for_each_relevant_impl(self.drop_trait_def_id, ty, |_| {
                self.drop_for_static_found = true;
            });
        }
        walk_path(self, path);
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}
