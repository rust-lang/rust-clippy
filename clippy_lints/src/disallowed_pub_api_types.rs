use clippy_config::Conf;
use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::paths::PathNS;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefIdMap;
use rustc_hir::intravisit::{Visitor, VisitorExt, walk_ty};
use rustc_hir::{AmbigArg, FnRetTy, ImplItem, ImplItemKind, Item, ItemKind, TraitItem, TraitItemKind, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::impl_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured types in clippy.toml from being used in the public API (function parameters or return types).
    ///
    /// ### Why is this bad?
    /// Some types are undesirable in public APIs (e.g. `anyhow::Result`).
    ///
    /// ### Example
    /// ```rust,ignore
    /// pub fn foo() -> anyhow::Result<()> {
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.80.0"]
    pub DISALLOWED_PUB_API_TYPES,
    style,
    "use of disallowed types in public API"
}

impl_lint_pass!(DisallowedPubApiTypes => [DISALLOWED_PUB_API_TYPES]);

pub struct DisallowedPubApiTypes {
    def_ids: DefIdMap<(&'static str, &'static DisallowedPath)>,
    prim_tys: FxHashMap<rustc_hir::PrimTy, (&'static str, &'static DisallowedPath)>,
}

impl DisallowedPubApiTypes {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (def_ids, prim_tys) = create_disallowed_map(
            tcx,
            &conf.disallowed_pub_api_types,
            PathNS::Type,
            def_kind_predicate,
            "type",
            true,
        );
        Self { def_ids, prim_tys }
    }

    fn check_res_emit(&self, cx: &LateContext<'_>, res: &Res, span: Span) {
        let (path, disallowed_path) = match res {
            Res::Def(_, did) if let Some(&x) = self.def_ids.get(did) => x,
            Res::PrimTy(prim) if let Some(&x) = self.prim_tys.get(prim) => x,
            _ => return,
        };
        span_lint_and_then(
            cx,
            DISALLOWED_PUB_API_TYPES,
            span,
            format!("using a disallowed type `{path}` in a public API"),
            disallowed_path.diag_amendment(span),
        );
    }
}

pub fn def_kind_predicate(def_kind: DefKind) -> bool {
    matches!(
        def_kind,
        DefKind::Struct
            | DefKind::Union
            | DefKind::Enum
            | DefKind::Trait
            | DefKind::TyAlias
            | DefKind::ForeignTy
            | DefKind::AssocTy
    )
}

struct TyVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    lint: &'a DisallowedPubApiTypes,
}

impl<'tcx> Visitor<'tcx> for TyVisitor<'_, 'tcx> {
    fn visit_ty(&mut self, ty: &'tcx Ty<'tcx, AmbigArg>) {
        if let TyKind::Path(path) = &ty.kind {
            let res = self.cx.qpath_res(path, ty.hir_id);
            self.lint.check_res_emit(self.cx, &res, ty.span);
        }
        walk_ty(self, ty);
    }
}

impl<'tcx> LateLintPass<'tcx> for DisallowedPubApiTypes {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Fn { ref sig, .. } = item.kind
            && cx.effective_visibilities.is_exported(item.owner_id.def_id)
        {
            let mut visitor = TyVisitor { cx, lint: self };
            for ty in sig.decl.inputs {
                visitor.visit_ty_unambig(ty);
            }
            if let FnRetTy::Return(ty) = sig.decl.output {
                visitor.visit_ty_unambig(ty);
            }
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Fn(ref sig, _) = item.kind
            && clippy_utils::trait_ref_of_method(cx, item.owner_id).is_none()
            && cx.effective_visibilities.is_exported(item.owner_id.def_id)
        {
            let mut visitor = TyVisitor { cx, lint: self };
            for ty in sig.decl.inputs {
                visitor.visit_ty_unambig(ty);
            }
            if let FnRetTy::Return(ty) = sig.decl.output {
                visitor.visit_ty_unambig(ty);
            }
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        if let TraitItemKind::Fn(ref sig, _) = item.kind
            && cx.effective_visibilities.is_exported(item.owner_id.def_id)
        {
            let mut visitor = TyVisitor { cx, lint: self };
            for ty in sig.decl.inputs {
                visitor.visit_ty_unambig(ty);
            }
            if let FnRetTy::Return(ty) = sig.decl.output {
                visitor.visit_ty_unambig(ty);
            }
        }
    }
}
