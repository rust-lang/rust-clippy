use clippy_config::Conf;
use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::paths::PathNS;
use clippy_utils::res::MaybeResPath;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{Expr, ExprKind, ItemKind, PathSegment, TyKind, UseKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::impl_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured modules in clippy.toml.
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// modules are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some modules are undesirable in certain contexts.
    ///
    /// ### Example:
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// disallowed-modules = [
    ///     # Can use a string as the path of the disallowed type.
    ///     "std::sync",
    ///     # Can also use an inline table with a `path` key.
    ///     { path = "std::net" },
    ///     # When using an inline table, can add a `reason` for why the type
    ///     # is disallowed.
    ///     { path = "std::sync", reason = "no std sync allowed" },
    ///     # Can also add a `replacement` that will be offered as a suggestion.
    ///     { path = "std::sync", reason = "prefer no_std guarded sync types", replacement = "crate::sync" },
    ///     # This would normally error if the path is incorrect, but with `allow-invalid` = `true`,
    ///     # it will be silently ignored
    ///     { path = "std::invalid::module", reason = "use alternative instead", allow-invalid = true }
    /// ]
    /// ```
    ///
    /// ```rust,ignore
    /// use std::sync;
    /// // or its use
    /// let x = std::sync::Mutext::default();
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // A similar module that is allowed by the config
    /// use crate::sync::Mutex;
    /// ```
    #[clippy::version = "1.70.0"]
    pub DISALLOWED_MODULES,
    style,
    "use of disallowed modules"
}

impl_lint_pass!(DisallowedModules => [DISALLOWED_MODULES]);

pub struct DisallowedModules {
    def_ids: DefIdMap<(&'static str, &'static DisallowedPath)>,
}

impl DisallowedModules {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (def_ids, _) = create_disallowed_map(
            tcx,
            &conf.disallowed_modules,
            PathNS::Type,
            def_kind_predicate,
            "module",
            false,
        );
        Self { def_ids }
    }

    fn check_did(&self, cx: &LateContext<'_>, did: DefId, span: Span) -> Option<()> {
        if let Some(&(path_str, disallowed_path)) = self.def_ids.get(&did) {
            span_lint_and_then(
                cx,
                DISALLOWED_MODULES,
                span,
                format!("use of a disallowed module `{path_str}`"),
                disallowed_path.diag_amendment(span),
            );
            None
        } else {
            Some(())
        }
    }

    fn check_path_segments<'a>(
        &self,
        cx: &LateContext<'a>,
        segments: impl Iterator<Item = &'a PathSegment<'a>>,
        span: Span,
    ) -> Option<()> {
        for segment in segments {
            if let Res::Def(def_kind, did) = segment.res
                && def_kind_predicate(def_kind)
            {
                self.check_did(cx, did, span)?;
            }
        }
        Some(())
    }

    fn check_did_all(&self, cx: &LateContext<'_>, did: DefId, span: Span) -> Option<()> {
        let mut current_id = Some(did);
        while let Some(did) = current_id {
            self.check_did(cx, did, span)?;
            current_id = cx.tcx.opt_parent(did);
        }

        Some(())
    }

    fn check_res_emit(&self, cx: &LateContext<'_>, res: &Res, span: Span) -> Option<()> {
        if let Some(did) = res.opt_def_id() {
            self.check_did_all(cx, did, span)?;
        }
        Some(())
    }
}

pub fn def_kind_predicate(def_kind: DefKind) -> bool {
    matches!(def_kind, DefKind::Mod)
}

impl<'tcx> LateLintPass<'tcx> for DisallowedModules {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'tcx>) {
        match item.kind {
            ItemKind::Use(path, UseKind::Single(_) | UseKind::Glob) => {
                if let Some(res) = path.res.type_ns {
                    self.check_res_emit(cx, &res, path.span);
                }
            },
            ItemKind::Impl(impl_trait) if let Some(trait_ref) = impl_trait.of_trait => {
                self.check_res_emit(cx, &trait_ref.trait_ref.path.res, item.span);
            },
            _ => {},
        }
    }

    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx rustc_hir::Ty<'tcx, rustc_hir::AmbigArg>) {
        match ty.kind {
            TyKind::Path(qpath) if let (_, Some(path)) = qpath.opt_res_path() => {
                if self
                    .check_path_segments(cx, path.segments.iter(), qpath.span())
                    .is_some()
                {
                    self.check_res_emit(cx, &path.res, qpath.span());
                }
            },
            _ => {},
        }
    }

    fn check_poly_trait_ref(&mut self, cx: &LateContext<'tcx>, poly: &'tcx rustc_hir::PolyTraitRef<'tcx>) {
        self.check_res_emit(cx, &poly.trait_ref.path.res, poly.trait_ref.path.span);
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::Match(_, arms, _) => {
                for arm in arms {
                    arm.pat.walk_always(|pat| {
                        if let (_, Some(path)) = pat.opt_res_path() {
                            self.check_res_emit(cx, &path.res, pat.span);
                        }
                    });
                }
            },
            ExprKind::Let(let_expr) => {
                let_expr.pat.walk_always(|pat| {
                    if let (_, Some(path)) = pat.opt_res_path() {
                        self.check_res_emit(cx, &path.res, pat.span);
                    }
                });
            },
            ExprKind::Path(qpath) if let (_, Some(path)) = qpath.opt_res_path() => {
                if self
                    .check_path_segments(cx, path.segments.iter(), qpath.span())
                    .is_some()
                {
                    self.check_res_emit(cx, &path.res, qpath.span());
                }
            },
            _ => {},
        }
    }
}
