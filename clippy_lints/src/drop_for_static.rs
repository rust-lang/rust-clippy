use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::has_drop;
use rustc_hir::intravisit::{Visitor, walk_path};
use rustc_hir::{HirId, Item, ItemKind, Path};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::hir::nested_filter;
use rustc_session::declare_lint_pass;
use rustc_span::Span;

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
    fn check_item<'a>(&mut self, cx: &LateContext<'a>, item: &'a Item<'a>) {
        if let ItemKind::Static(_, ident, ty, _) = item.kind
            && let Some(ty_amb) = ty.try_as_ambig_ty()
        {
            let mut visitor = DropForStaticVisitor::new(cx);
            visitor.visit_ty(ty_amb);
            if let Some(type_with_drop_span) = visitor.type_with_drop_span {
                span_lint_and_then(
                    cx,
                    DROP_FOR_STATIC,
                    ident.span,
                    "static items with drop implementation",
                    |diag| {
                        diag.span_label(type_with_drop_span, "type with drop implementation here");
                    },
                );
            }
        }
    }
}

struct DropForStaticVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    type_with_drop_span: Option<Span>,
}
impl<'a, 'tcx> DropForStaticVisitor<'a, 'tcx> {
    fn new(cx: &'a LateContext<'tcx>) -> Self {
        Self {
            cx,
            type_with_drop_span: None,
        }
    }
}

impl<'tcx> Visitor<'tcx> for DropForStaticVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::All;

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) {
        if let Some(def_id) = path.res.opt_def_id() {
            let ty = self.cx.tcx.type_of(def_id).instantiate_identity();
            if has_drop(self.cx, ty) {
                self.type_with_drop_span = Some(path.span);
            } else {
                walk_path(self, path);
            }
        }
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}
