use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, HirId, LangItem, Pat, PatExpr, PatExprKind, PatKind, QPath, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty::TypeckResults;
use rustc_middle::ty::layout::HasTyCtxt;
use rustc_span::Symbol;

/// A `QPath` with the `HirId` of the node containing it.
type QPathId<'tcx> = (&'tcx QPath<'tcx>, HirId);

/// A HIR node which might be a `QPath`.
pub trait MaybeQPath<'tcx> {
    /// If this node is a path gets both the contained path and the `HirId` to
    /// use for type dependant lookup.
    fn opt_qpath(self) -> Option<QPathId<'tcx>>;
}

impl<'tcx> MaybeQPath<'tcx> for QPathId<'tcx> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        Some((self.0, self.1))
    }
}
impl<'tcx> MaybeQPath<'tcx> for &'tcx Expr<'_> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        match &self.kind {
            ExprKind::Path(qpath) => Some((qpath, self.hir_id)),
            _ => None,
        }
    }
}
impl<'tcx> MaybeQPath<'tcx> for &'tcx PatExpr<'_> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        match &self.kind {
            PatExprKind::Path(qpath) => Some((qpath, self.hir_id)),
            _ => None,
        }
    }
}
impl<'tcx, AmbigArg> MaybeQPath<'tcx> for &'tcx Ty<'_, AmbigArg> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        match &self.kind {
            TyKind::Path(qpath) => Some((qpath, self.hir_id)),
            _ => None,
        }
    }
}
impl<'tcx> MaybeQPath<'tcx> for &'_ Pat<'tcx> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        match self.kind {
            PatKind::Expr(e) => e.opt_qpath(),
            _ => None,
        }
    }
}
impl<'tcx, T: MaybeQPath<'tcx>> MaybeQPath<'tcx> for Option<T> {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        self.and_then(T::opt_qpath)
    }
}
impl<'tcx, T: Copy + MaybeQPath<'tcx>> MaybeQPath<'tcx> for &'_ T {
    #[inline]
    fn opt_qpath(self) -> Option<QPathId<'tcx>> {
        T::opt_qpath(*self)
    }
}

/// A type which contains the results of type dependant name resolution.
///
/// All the functions on this trait will lookup the path's resolution. This lookup
/// is not free and should be done at most once per item. e.g.
///
/// ```ignore
/// // Don't do this
/// let is_option_ctor = item.is_path_lang_item(tcx, LangItem::OptionSome)
///     || item.is_path_lang_item(tcx, LangItem::OptionNone);
///
/// // Prefer this
/// let is_option_ctor = item.path_def_id().is_some_and(|did| {
///     tcx.lang_items().option_none_variant() == Some(did)
///         || tcx.lang_items().option_some_variant() == Some(did)
/// });
/// ```
pub trait PathRes<'tcx> {
    /// Gets the definition a node resolves to if it has a type dependent resolution.
    fn type_dependent_def(&self, id: HirId) -> Option<(DefKind, DefId)>;

    /// Gets the resolution of a node if it has a type dependent resolution. Returns
    /// `Res::Err` otherwise.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn type_dependent_res(&self, id: HirId) -> Res {
        self.type_dependent_def(id)
            .map_or(Res::Err, |(kind, id)| Res::Def(kind, id))
    }

    /// Gets the resolution of the path.
    ///
    /// `id` must be the `HirId` of the node containing `qpath`.
    #[cfg_attr(debug_assertions, track_caller)]
    fn qpath_res(&self, qpath: &QPath<'_>, id: HirId) -> Res {
        match qpath {
            QPath::Resolved(_, p) => p.res,
            QPath::TypeRelative(..) | QPath::LangItem(..) => self.type_dependent_res(id),
        }
    }

    /// Gets the resolution of the item if it's a path. Returns `Res::Err` otherwise.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn path_res<'a>(&self, path: impl MaybeQPath<'a>) -> Res {
        match path.opt_qpath() {
            Some((qpath, hir_id)) => self.qpath_res(qpath, hir_id),
            None => Res::Err,
        }
    }

    /// Gets the definition the given node resolves to.
    #[cfg_attr(debug_assertions, track_caller)]
    fn path_def<'a>(&self, path: impl MaybeQPath<'a>) -> Option<(DefKind, DefId)> {
        match path.opt_qpath() {
            Some((&QPath::Resolved(_, p), _)) => match p.res {
                Res::Def(kind, id) => Some((kind, id)),
                _ => None,
            },
            Some((QPath::TypeRelative(..) | QPath::LangItem(..), id)) => self.type_dependent_def(id),
            _ => None,
        }
    }

    /// Gets the `DefId` of the item the given node resolves to.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn path_def_id<'a>(&self, path: impl MaybeQPath<'a>) -> Option<DefId> {
        self.path_def(path).map(|(_, id)| id)
    }

    /// Checks if the path resolves to the specified item.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn is_path_item<'a>(&self, path: impl MaybeQPath<'a>, did: DefId) -> bool {
        self.path_def_id(path) == Some(did)
    }

    /// Gets the diagnostic name of the item the given node resolves to.
    #[cfg_attr(debug_assertions, track_caller)]
    fn path_diag_name<'a>(&self, path: impl MaybeQPath<'a>) -> Option<Symbol>
    where
        Self: HasTyCtxt<'tcx>,
    {
        self.path_def_id(path)
            .and_then(|did| self.tcx().get_diagnostic_name(did))
    }

    /// Checks if the path resolves to the specified diagnostic item.
    #[cfg_attr(debug_assertions, track_caller)]
    fn is_path_diag_item<'a>(&self, path: impl MaybeQPath<'a>, name: Symbol) -> bool
    where
        Self: HasTyCtxt<'tcx>,
    {
        self.path_def_id(path)
            .is_some_and(|did| self.tcx().is_diagnostic_item(name, did))
    }

    /// Checks if the path resolves to the specified `LangItem`.
    #[cfg_attr(debug_assertions, track_caller)]
    fn is_path_lang_item<'a>(&self, path: impl MaybeQPath<'a>, item: LangItem) -> bool
    where
        Self: HasTyCtxt<'tcx>,
    {
        self.path_def_id(path)
            .is_some_and(|did| self.tcx().lang_items().get(item) == Some(did))
    }

    /// If the path resolves to a constructor, gets the `DefId` of the corresponding struct/variant.
    #[cfg_attr(debug_assertions, track_caller)]
    fn path_ctor_parent_id<'a>(&self, path: impl MaybeQPath<'a>) -> Option<DefId>
    where
        Self: HasTyCtxt<'tcx>,
    {
        if let Res::Def(DefKind::Ctor(..), id) = self.path_res(path) {
            self.tcx().opt_parent(id)
        } else {
            None
        }
    }

    /// Checks if the path resolves to the constructor of the specified `LangItem`.
    #[cfg_attr(debug_assertions, track_caller)]
    fn is_path_lang_ctor<'a>(&self, path: impl MaybeQPath<'a>, item: LangItem) -> bool
    where
        Self: HasTyCtxt<'tcx>,
    {
        self.path_ctor_parent_id(path)
            .is_some_and(|did| self.tcx().lang_items().get(item) == Some(did))
    }
}
impl<'tcx> PathRes<'tcx> for LateContext<'tcx> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn type_dependent_def(&self, id: HirId) -> Option<(DefKind, DefId)> {
        if let Some(typeck) = self.maybe_typeck_results() {
            PathRes::type_dependent_def(typeck, id)
        } else {
            // It's possible to get the `TypeckResults` for any other body, but
            // attempting to lookup the type of something across bodies like this
            // is a good indication of a bug.
            debug_assert!(false, "attempted type-dependent lookup in a non-body context");
            None
        }
    }
}
impl PathRes<'_> for TypeckResults<'_> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn type_dependent_def(&self, id: HirId) -> Option<(DefKind, DefId)> {
        if id.owner == self.hir_owner {
            self.type_dependent_def(id)
        } else {
            debug_assert!(
                false,
                "attempted type-dependent lookup for a node in the wrong body.\n  in body `{:?}`\n  expected body `{:?}`",
                self.hir_owner, id.owner,
            );
            None
        }
    }
}
