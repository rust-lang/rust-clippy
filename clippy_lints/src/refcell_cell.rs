use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeResPath as _};
use clippy_utils::source::{snippet_with_applicability, snippet_with_context};
use clippy_utils::ty::{approx_ty_size, is_copy, ty_from_hir_ty};
use clippy_utils::{is_trait_impl_item, qpath_generic_tys, sym};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    self, Body, Expr, ExprKind, FieldDef, FnDecl, FnRetTy, GenericArg, ImplItem, ImplItemKind, Item, ItemKind, QPath,
    Stmt, StmtKind, TraitItem, TraitItemKind, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::sym::RefCell;

declare_clippy_lint! {
    /// ### What it does
    /// This lint suggests using `Cell` for `Copy` types instead of `RefCell`.
    ///
    /// ### Why is this bad?
    /// `RefCell` avoids cloning at the cost of additional memory usage and
    /// instructions, which isn't worth it for **small** `Copy` types.
    ///
    /// ### Known problems
    /// `RefCell` might be useful for **large** `Copy` types.
    /// See <https://manishearth.github.io/blog/2015/05/27/wrapper-types-in-rust-choosing-your-guarantees/#cell-types> for details.
    ///
    /// ### Example
    /// ```no_run
    /// use std::cell::RefCell;
    ///
    /// struct Counter(RefCell<i32>);
    ///
    /// let _ = RefCell::new(1);
    /// let _ = RefCell::<i32>::default();
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::cell::Cell;
    ///
    /// struct Counter(Cell<i32>);
    ///
    /// let _ = Cell::new(1);
    /// let _ = Cell::<i32>::default();
    /// ```
    #[clippy::version = "1.99.0"]
    pub REFCELL_CELL,
    perf,
    "using a `RefCell` for a `Copy` type"
}

impl_lint_pass!(RefcellCell => [REFCELL_CELL]);

pub struct RefcellCell {
    max_cheap_copy_size: u64,
}

impl RefcellCell {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            max_cheap_copy_size: conf.max_cheap_copy_size,
        }
    }

    fn emit_refcell_copy_def<'tcx>(&self, cx: &LateContext<'tcx>, hir_ty: &'tcx Ty<'tcx>) {
        match hir_ty.kind {
            TyKind::Array(hir_ty, _) | TyKind::Slice(hir_ty) => {
                self.emit_refcell_copy_def(cx, hir_ty);
            },
            TyKind::Tup(hir_tys) => {
                for hir_ty in hir_tys {
                    self.emit_refcell_copy_def(cx, hir_ty);
                }
            },
            TyKind::Path(qpath)
                if let (None, Some(path)) = qpath.opt_res_path()
                // RefCell<T>
                && let [segment] = path.segments
                && segment.res.is_diag_item(&cx.tcx, RefCell)
                && let Some(args) = segment.args
                && let [GenericArg::Type(hir_ty)] = args.args
                // `T` is copyable and small
                && {
                    let ty = ty_from_hir_ty(cx, hir_ty.as_unambig_ty());
                    is_copy(cx, ty) && approx_ty_size(cx, ty) <= self.max_cheap_copy_size
                }
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
}

impl<'tcx> LateLintPass<'tcx> for RefcellCell {
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, def: &'tcx FieldDef<'tcx>) {
        if !def.span.from_expansion() {
            self.emit_refcell_copy_def(cx, def.ty);
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
                self.emit_refcell_copy_def(cx, hir_ty);
            }
            if let FnRetTy::Return(hir_ty) = decl.output {
                self.emit_refcell_copy_def(cx, hir_ty);
            }
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Type(hir_ty) = item.kind
            && !item.span.from_expansion()
        {
            self.emit_refcell_copy_def(cx, hir_ty);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::TyAlias(_, _, hir_ty) = item.kind
            && !item.span.from_expansion()
        {
            self.emit_refcell_copy_def(cx, hir_ty);
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        if let TraitItemKind::Fn(sig, _) = item.kind
            && !item.span.from_expansion()
        {
            for hir_ty in sig.decl.inputs {
                self.emit_refcell_copy_def(cx, hir_ty);
            }
            if let FnRetTy::Return(hir_ty) = sig.decl.output {
                self.emit_refcell_copy_def(cx, hir_ty);
            }
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(let_stmt) = stmt.kind
            && !stmt.span.from_expansion()
            && let Some(init) = let_stmt.init
            // RefCell<T>
            && let Some(ty) = cx.typeck_results().expr_ty_opt(init)
            && let ty::Adt(def, args) = ty.kind()
            && def.is_diag_item(&cx.tcx, RefCell)
            // `T` is a small `Copy` type
            && is_copy(cx, args.type_at(0))
            && approx_ty_size(cx, args.type_at(0)) <= self.max_cheap_copy_size
            // `init` is a constructor of `RefCell`
            && let ExprKind::Call(maybe_qpath, args) = init.kind
            && let Some(kind) = CtorKind::new(cx, maybe_qpath, args.len())
            // Be conservative about macro
            && args.iter().all(|e| !e.span.from_expansion())
        {
            // This can be a false positive, for example, when passed to a function
            // that requires `RefCell`.
            let mut app = Applicability::MaybeIncorrect;

            let (span, sugg) = if let Some(annotation) = let_stmt.ty
                && let TyKind::Path(qpath) = annotation.kind
                && let Some(inner_ty) = qpath_generic_tys(&qpath).next()
            {
                let sugg = {
                    let inner_ty = snippet_with_context(cx, inner_ty.span, stmt.span.ctxt(), "_", &mut app).0;

                    let arg = args
                        .first()
                        .map_or_default(|arg| snippet_with_context(cx, arg.span, stmt.span.ctxt(), "..", &mut app).0);
                    let init = match kind {
                        CtorKind::New => format!("Cell::new({arg})"),
                        CtorKind::From => format!("Cell::from({arg})"),
                        CtorKind::Default => "Cell::default()".to_string(),
                    };

                    format!(/* let pat: */ "Cell<{inner_ty}> = {init};")
                };

                (annotation.span.to(init.span), sugg)
            } else {
                let sugg = {
                    let arg = args
                        .first()
                        .map_or_default(|arg| snippet_with_context(cx, arg.span, stmt.span.ctxt(), "..", &mut app).0);
                    let init = match kind {
                        CtorKind::New => format!("Cell::new({arg})"),
                        CtorKind::From => format!("Cell::from({arg})"),
                        CtorKind::Default => "Cell::default()".to_string(),
                    };

                    let extract_ty = |qpath| {
                        // <T>::ctor
                        let ty = match qpath {
                            QPath::Resolved(ty, _) => ty,
                            QPath::TypeRelative(ty, _) => Some(ty),
                        };

                        if let Some(TyKind::Path(qpath)) = ty.map(|ty| ty.kind)
                            && let Some(ty) = qpath_generic_tys(&qpath).next()
                        {
                            Some(ty)
                        } else {
                            None
                        }
                    };

                    if let ExprKind::Path(qpath) = maybe_qpath.kind
                        && let Some(ty) = extract_ty(qpath)
                    {
                        let ty = snippet_with_context(cx, ty.span, stmt.span.ctxt(), "_", &mut app).0;
                        init.replacen("::", &format!("::<{ty}>::"), 1)
                    } else {
                        init
                    }
                };

                (init.span, sugg)
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

enum CtorKind {
    New,
    From,
    Default,
}

impl CtorKind {
    fn new<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, num_args: usize) -> Option<Self> {
        if let Some(def_id) = expr.res(cx).opt_def_id()
            && def_id.is_diag_item(&cx.tcx, sym::from_fn)
            && num_args == 1
        {
            Some(Self::From)
        } else if let Some(def_id) = expr.res(cx).opt_def_id()
            && def_id.is_diag_item(&cx.tcx, sym::default_fn)
            && num_args == 0
        {
            Some(Self::Default)
        } else if let ExprKind::Path(QPath::TypeRelative(hir_ty, segment)) = expr.kind
            && segment.ident.name == sym::new
            && hir_ty.res(cx).is_diag_item(&cx.tcx, RefCell)
            && num_args == 1
        {
            Some(Self::New)
        } else {
            None
        }
    }
}
