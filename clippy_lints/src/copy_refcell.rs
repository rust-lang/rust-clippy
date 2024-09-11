use clippy_config::Conf;
use rustc_hir::{FieldDef, LetStmt, LocalSource};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_middle::ty::layout::TyAndLayout;
use rustc_session::impl_lint_pass;
use rustc_span::symbol::sym;
use rustc_span::Span;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::ty::implements_trait;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects crate-local usage of `RefCell<T>` where `T` implements `Copy`
    ///
    /// ### Why is this bad?
    ///
    /// `RefCell` has to perform extra book-keeping in order to support references, whereas `Cell` does not.
    ///
    /// ### Example
    /// ```no_run
    /// let interior_mutable = std::cell::RefCell::new(0_u8);
    ///
    /// *interior_mutable.borrow_mut() = 1;
    /// ```
    /// Use instead:
    /// ```no_run
    /// let interior_mutable = std::cell::Cell::new(0_u8);
    ///
    /// interior_mutable.set(1);
    /// ```
    #[clippy::version = "1.83.0"]
    pub COPY_REFCELL,
    pedantic,
    "usage of `RefCell<T>` where `T` implements `Copy`"
}

pub struct CopyRefCell {
    maximum_size: u64,
    avoid_breaking_exported_api: bool,
}

impl CopyRefCell {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            maximum_size: conf.large_cell_limit,
            avoid_breaking_exported_api: conf.avoid_breaking_exported_api,
        }
    }

    fn check_copy_refcell<'tcx>(&self, cx: &LateContext<'tcx>, ty: ty::Ty<'tcx>, span: Span) {
        let Some(copy_def_id) = cx.tcx.get_diagnostic_item(sym::Copy) else {
            return;
        };

        let ty::Adt(adt, generics) = ty.kind() else {
            return;
        };

        if !cx.tcx.is_diagnostic_item(sym::RefCell, adt.did()) {
            return;
        }

        let inner_ty = generics.type_at(0);
        let Ok(TyAndLayout { layout, .. }) = cx.tcx.layout_of(cx.param_env.and(inner_ty)) else {
            return;
        };

        if layout.size().bytes() >= self.maximum_size {
            return;
        }

        if implements_trait(cx, inner_ty, copy_def_id, &[]) {
            span_lint_and_help(
                cx,
                COPY_REFCELL,
                span,
                "`RefCell` used with a type that implements `Copy`",
                None,
                "replace the usage of `RefCell` with `Cell`, which does not have to track borrowing at runtime",
            );
        }
    }
}

impl_lint_pass!(CopyRefCell => [COPY_REFCELL]);

impl<'tcx> LateLintPass<'tcx> for CopyRefCell {
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field_def: &'tcx FieldDef<'tcx>) {
        // Don't want to lint macro expansions.
        if field_def.span.from_expansion() {
            return;
        }

        if self.avoid_breaking_exported_api && cx.effective_visibilities.is_exported(field_def.def_id) {
            return;
        }

        let field_ty = rustc_hir_analysis::lower_ty(cx.tcx, field_def.ty);
        self.check_copy_refcell(cx, field_ty, field_def.ty.span);
    }

    fn check_local(&mut self, cx: &LateContext<'tcx>, local_def: &'tcx LetStmt<'tcx>) {
        // Don't want to lint macro expansions or desugaring.
        if local_def.span.from_expansion() || !matches!(local_def.source, LocalSource::Normal) {
            return;
        }

        let Some(init_expr) = local_def.init else {
            return;
        };

        let init_ty = cx.typeck_results().expr_ty(init_expr);
        self.check_copy_refcell(cx, init_ty, init_expr.span);
    }
}
