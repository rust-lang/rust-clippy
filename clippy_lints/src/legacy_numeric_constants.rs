use clippy_utils::diagnostics::{span_lint_and_then, span_lint_hir_and_then};
use clippy_utils::msrvs::{Msrv, NUMERIC_ASSOCIATED_CONSTANTS};
use clippy_utils::source::snippet_opt;
use clippy_utils::{get_parent_expr, is_from_proc_macro, last_path_segment, std_or_core};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{walk_expr, Visitor};
use rustc_hir::{Expr, ExprKind, Item, ItemKind, PrimTy, QPath, TyKind, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::symbol::kw;
use rustc_span::{sym, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `<integer>::max_value()`, `std::<integer>::MAX`,
    /// `std::<float>::EPSILON`, etc.
    ///
    /// ### Why is this bad?
    /// All of these have been superceded by the associated constants on their respective types,
    /// such as `i128::MAX`. These legacy items may be deprecated in a future version of rust.
    ///
    /// ### Example
    /// ```rust
    /// let eps = std::f32::EPSILON;
    /// ```
    /// Use instead:
    /// ```rust
    /// let eps = f32::EPSILON;
    /// ```
    #[clippy::version = "1.72.0"]
    pub LEGACY_NUMERIC_CONSTANTS,
    style,
    "checks for usage of legacy std numeric constants and methods"
}
pub struct LegacyNumericConstants {
    msrv: Msrv,
}

impl LegacyNumericConstants {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(LegacyNumericConstants => [LEGACY_NUMERIC_CONSTANTS]);

impl<'tcx> LateLintPass<'tcx> for LegacyNumericConstants {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Self { msrv } = self;

        if msrv.meets(NUMERIC_ASSOCIATED_CONSTANTS)
            && !in_external_macro(cx.sess(), item.span)
            && let ItemKind::Use(path, kind @ (UseKind::Single | UseKind::Glob)) = item.kind
            // These modules are "TBD" deprecated, and the contents are too, so lint on the `use`
            // statement directly
            && let def_path = cx.get_def_path(path.res[0].def_id())
            && is_path_in_numeric_module(&def_path, true)
        {
            let plurality = matches!(
                kind,
                UseKind::Glob | UseKind::Single if matches!(path.res[0], Res::Def(DefKind::Mod, _)),
            );

            span_lint_and_then(
                cx,
                LEGACY_NUMERIC_CONSTANTS,
                path.span,
                if plurality {
                    "importing legacy numeric constants"
                } else {
                    "importing a legacy numeric constant"
                },
                |diag| {
                    if item.ident.name != kw::Underscore {
                        let msg = if plurality && let [.., module_name] = &*def_path {
                            format!("use the associated constants on `{module_name}` instead at their usage")
                        } else if let [.., module_name, name] = &*def_path {
                            format!("use the associated constant `{module_name}::{name}` instead at its usage")
                        } else {
                            return;
                        };

                        diag.help(msg);
                    }
                },
            );
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let Self { msrv } = self;

        cx.tcx.hir().visit_all_item_likes_in_crate(&mut V { cx, msrv });
    }

    extract_msrv_attr!(LateContext);
}

struct V<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    msrv: &'a Msrv,
}

impl<'tcx> Visitor<'tcx> for V<'_, 'tcx> {
    type NestedFilter = OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        walk_expr(self, expr);
        let Self { cx, msrv } = *self;

        if !msrv.meets(NUMERIC_ASSOCIATED_CONSTANTS) || in_external_macro(cx.sess(), expr.span) {
            return;
        }
        let ExprKind::Path(qpath) = expr.kind else {
            return;
        };

        // `std::<integer>::<CONST>` check
        let (span, sugg, msg) = if let QPath::Resolved(None, path) = qpath
            && let Some(def_id) = path.res.opt_def_id()
            && let path = cx.get_def_path(def_id)
            && is_path_in_numeric_module(&path, false)
            && let [.., module_name, name] = &*path
            && let Some(snippet) = snippet_opt(cx, expr.span)
            && let is_float_module = (*module_name == sym::f32 || *module_name == sym::f64)
            // Skip linting if this usage looks identical to the associated constant, since this
            // would only require removing a `use` import. We don't ignore ones from `f32` or `f64`, however.
            && let identical = snippet == format!("{module_name}::{name}")
            && (!identical || is_float_module)
        {
            (
                expr.span,
                if identical {
                    None
                } else {
                    Some(format!("{module_name}::{name}"))
                },
                "usage of a legacy numeric constant",
            )
        // `<integer>::xxx_value` check
        } else if let QPath::TypeRelative(ty, _) = qpath
            && let TyKind::Path(ty_qpath) = ty.kind
            && let Res::PrimTy(PrimTy::Int(_) | PrimTy::Uint(_)) = cx.qpath_res(&ty_qpath, ty.hir_id)
            && let last_segment = last_path_segment(&qpath)
            && let name = last_segment.ident.name.as_str()
            && (name == "max_value" || name == "min_value")
            // Also remove the `()`
            && let Some(par_expr) = get_parent_expr(cx, expr)
            && let ExprKind::Call(_, _) = par_expr.kind
        {
            (
                qpath.last_segment_span().with_hi(par_expr.span.hi()),
                Some(name[..=2].to_ascii_uppercase()),
                "usage of a legacy numeric method",
            )
        } else {
            return;
        };

        if !is_from_proc_macro(cx, expr) {
            span_lint_hir_and_then(cx, LEGACY_NUMERIC_CONSTANTS, expr.hir_id, span, msg, |diag| {
                if let Some(sugg) = sugg {
                    diag.span_suggestion(
                        span,
                        "use the associated constant instead",
                        sugg,
                        Applicability::MaybeIncorrect,
                    );
                } else if let Some(std_or_core) = std_or_core(cx)
                    && let QPath::Resolved(None, path) = qpath
                {
                    diag.help(format!(
                        "remove the import that brings `{std_or_core}::{}` into scope",
                        // Must be `<module>::<CONST>` if `needs_import_removed` is true yet is
                        // being linted
                        path.segments[0].ident.name,
                    ));
                }
            });
        }
    }
}

fn is_path_in_numeric_module(path: &[Symbol], ignore_float_modules: bool) -> bool {
    if let [
        sym::core,
        module @ (sym::u8
        | sym::i8
        | sym::u16
        | sym::i16
        | sym::u32
        | sym::i32
        | sym::u64
        | sym::i64
        | sym::u128
        | sym::i128
        | sym::usize
        | sym::isize
        | sym::f32
        | sym::f64),
        ..,
    ] = path
        && !path.get(2).is_some_and(|&s| s == sym!(consts))
    {
        // If `ignore_float_modules` is `true`, return `None` for `_::f32` or `_::f64`, but not
        // _::f64::MAX` or similar.
        if ignore_float_modules && (*module == sym::f32 || *module == sym::f64) && path.get(2).is_none() {
            return false;
        }

        return true;
    }

    false
}
