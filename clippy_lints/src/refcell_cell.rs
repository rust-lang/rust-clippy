use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeResPath as _};
use clippy_utils::source::{snippet, snippet_with_applicability, snippet_with_context};
use clippy_utils::ty::{is_copy, ty_from_hir_ty};
use clippy_utils::{is_trait_impl_item, sym};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    self, Body, Expr, ExprKind, FieldDef, FnDecl, FnRetTy, GenericArg, ImplItem, ImplItemKind, Item, ItemKind, QPath,
    Stmt, StmtKind, TraitItem, TraitItemKind, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::sym::RefCell;

declare_clippy_lint! {
    /// ### What it does
    /// This lint suggests using `Cell` for `Copy` types instead of `RefCell`.
    ///
    /// ### Why is this bad?
    /// `RefCell` avoids cloning at the cost of additional memory usage and
    /// instructions, which isn't worth it for `Copy` types.
    ///
    /// ### Example
    /// ```no_run
    /// use std::cell::RefCell;
    ///
    /// struct Counter(RefCell<i32>);
    ///
    /// let _ = RefCell::new(1);
    /// let _ = RefCell::default();
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::cell::Cell;
    ///
    /// struct Counter(Cell<i32>);
    ///
    /// let _ = Cell::new(1);
    /// let _ = Cell::default();
    /// ```
    #[clippy::version = "1.99.0"]
    pub REFCELL_CELL,
    perf,
    "using a `RefCell` for a `Copy` type"
}

declare_lint_pass!(RefcellCell => [REFCELL_CELL]);

impl<'tcx> LateLintPass<'tcx> for RefcellCell {
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, def: &'tcx FieldDef<'tcx>) {
        if !def.span.from_expansion() {
            emit_refcell_copy_def(cx, def.ty);
        }
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        _: &'tcx Body<'tcx>,
        span: Span,
        def_id: LocalDefId,
    ) {
        if !span.from_expansion() && !is_trait_impl_item(cx, cx.tcx.local_def_id_to_hir_id(def_id)) {
            for hir_ty in decl.inputs {
                emit_refcell_copy_def(cx, hir_ty);
            }
            if let FnRetTy::Return(hir_ty) = decl.output {
                emit_refcell_copy_def(cx, hir_ty);
            }
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Type(hir_ty) = item.kind
            && !item.span.from_expansion()
        {
            emit_refcell_copy_def(cx, hir_ty);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::TyAlias(_, _, hir_ty) = item.kind
            && !item.span.from_expansion()
        {
            emit_refcell_copy_def(cx, hir_ty);
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        if let TraitItemKind::Fn(sig, _) = item.kind
            && !item.span.from_expansion()
        {
            for hir_ty in sig.decl.inputs {
                emit_refcell_copy_def(cx, hir_ty);
            }
            if let FnRetTy::Return(hir_ty) = sig.decl.output {
                emit_refcell_copy_def(cx, hir_ty);
            }
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(let_stmt) = stmt.kind
            && !stmt.span.from_expansion()
            && let Some(init) = let_stmt.init
            // RefCell<T: Copy>
            && let Some(ty) = cx.typeck_results().expr_ty_opt(init)
            && let ty::Adt(def,args) = ty.kind()
            && def.is_diag_item(&cx.tcx, RefCell)
            && is_copy(cx, args.type_at(0))
            // `init` is a constructor of `RefCell`
            && let ExprKind::Call(maybe_qpath, args) = init.kind
            && is_refcell_ctor(cx, maybe_qpath, args.len())
            // Be conservative about macro
            && args.iter().all(|e| !e.span.from_expansion())
        {
            let span = match (
                snippet(cx, init.span, "").contains("RefCell"),
                let_stmt
                    .ty
                    .is_some_and(|ty| snippet(cx, ty.span, "").contains("RefCell")),
            ) {
                (true, true) => let_stmt.span,
                (true, false) => init.span,
                (false, true) => let_stmt.ty.unwrap().span,
                (false, false) => return,
            };

            // Because this may be required to be `RefCell`
            let mut app = Applicability::MaybeIncorrect;
            let sugg = {
                let (init, _) = snippet_with_context(cx, span, stmt.span.ctxt(), "..", &mut app);
                init.into_owned().replace("RefCell", "Cell")
            };
            span_lint_and_sugg(
                cx,
                REFCELL_CELL,
                span,
                "using a `RefCell` for a `Copy` type",
                "try",
                sugg,
                app,
            );
        }
    }
}

fn emit_refcell_copy_def<'tcx>(cx: &LateContext<'tcx>, hir_ty: &'tcx Ty<'tcx>) {
    match hir_ty.kind {
        TyKind::Array(hir_ty, _) | TyKind::Slice(hir_ty) => {
            emit_refcell_copy_def(cx, hir_ty);
        },
        TyKind::Tup(hir_tys) => {
            for hir_ty in hir_tys {
                emit_refcell_copy_def(cx, hir_ty);
            }
        },
        TyKind::Path(qpath)
            if let (None, Some(path)) = qpath.opt_res_path()
                // RefCell<T>
                && let [segment] = path.segments
                && segment.res.is_diag_item(&cx.tcx, RefCell)
                && let Some(args) = segment.args
                && let [GenericArg::Type(hir_ty)] = args.args
                // T: Copy
                && is_copy(cx, ty_from_hir_ty(cx, hir_ty.as_unambig_ty()))
                // Be conservative about macro
                && !hir_ty.span.from_expansion() =>
        {
            let mut app = Applicability::MaybeIncorrect;
            let sugg = {
                let hir_ty = snippet_with_applicability(cx, hir_ty.span, "_", &mut app);
                format!("Cell<{hir_ty}>")
            };
            span_lint_and_sugg(
                cx,
                REFCELL_CELL,
                path.span,
                "using a `RefCell` for a `Copy` type",
                "try",
                sugg,
                app,
            );
        },
        _ => (),
    }
}

fn is_refcell_ctor<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, num_args: usize) -> bool {
    if let Some(def_id) = expr.res(cx).opt_def_id()
        && let Some(name) = cx.tcx.get_diagnostic_name(def_id)
        && (matches!(name, sym::from_fn if num_args == 1) || matches!(name, sym::default_fn if num_args == 0))
    {
        return true;
    }

    // RefCell::new()
    if let ExprKind::Path(QPath::TypeRelative(hir_ty, segment)) = expr.kind
        && segment.ident.name == sym::new
        && hir_ty.res(cx).is_diag_item(&cx.tcx, RefCell)
        && num_args == 1
    {
        return true;
    }

    false
}
