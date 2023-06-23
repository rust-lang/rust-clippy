use clippy_utils::{
    diagnostics::span_lint_and_then,
    get_parent_expr, is_from_proc_macro, last_path_segment,
    msrvs::{self, Msrv},
};
use itertools::Itertools;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::{
    def::Res,
    def_id::DefId,
    intravisit::{walk_expr, Visitor},
    Item, UseKind,
};
use rustc_hir::{Expr, ExprKind, ItemKind, PrimTy, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{hir::nested_filter::OnlyBodies, lint::in_external_macro};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `<integer>::max_value()`, `std::<integer>::MAX`,
    /// `std::<float>::EPSILON`, etc.
    ///
    /// ### Why is this bad?
    /// All of these have been superceded by the associated constants on their respective types,
    /// such as `i128::MAX`. These legacy constants may be deprecated in a future version of rust.
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
    use_stmts: FxHashMap<Symbol, Vec<Span>>,
    glob_use_stmts: Vec<Span>,
}

impl LegacyNumericConstants {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self {
            msrv,
            use_stmts: FxHashMap::default(),
            glob_use_stmts: vec![],
        }
    }
}

impl_lint_pass!(LegacyNumericConstants => [LEGACY_NUMERIC_CONSTANTS]);

impl<'tcx> LateLintPass<'tcx> for LegacyNumericConstants {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Self {
            msrv: _,
            use_stmts,
            glob_use_stmts,
        } = self;

        if !item.span.is_dummy() && let ItemKind::Use(path, kind) = item.kind {
            match kind {
                UseKind::Single => {
                    for res in &path.res {
                        if let Some(def_id) = res.opt_def_id()
                            && let Some(module_name) = is_path_in_integral_module(cx, def_id)
                            && let _ = use_stmts.insert(module_name, vec![])
                            && let Some(use_stmts) = use_stmts.get_mut(&module_name)
                        {
                            use_stmts.push(item.span);
                        }
                    }
                },
                UseKind::Glob => glob_use_stmts.push(item.span),
                UseKind::ListStem => {},
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let Self {
            msrv,
            use_stmts,
            glob_use_stmts,
        } = self;

        let mut v = V {
            cx,
            msrv,
            use_stmts,
            glob_use_stmts,
        };
        cx.tcx.hir().visit_all_item_likes_in_crate(&mut v);
    }

    extract_msrv_attr!(LateContext);
}

struct V<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    msrv: &'a Msrv,
    use_stmts: &'a FxHashMap<Symbol, Vec<Span>>,
    glob_use_stmts: &'a Vec<Span>,
}

impl<'tcx> Visitor<'tcx> for V<'_, 'tcx> {
    type NestedFilter = OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.cx.tcx.hir()
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        walk_expr(self, expr);
        let Self {
            cx,
            msrv,
            use_stmts,
            glob_use_stmts,
        } = *self;

        if !msrv.meets(msrvs::STD_INTEGRAL_CONSTANTS) || in_external_macro(cx.sess(), expr.span) {
            return;
        }
        let ExprKind::Path(qpath) = expr.kind else {
            return;
        };

        // `std::<integer>::<CONST>` check
        let (span, sugg, is_method, use_stmts) = if let QPath::Resolved(_, path) = qpath
            && let Some(def_id) = path.res.opt_def_id()
            && let Some(name) = path.segments.iter().last().map(|segment| segment.ident.name)
            && let Some(module_name) = is_path_in_integral_module(cx, def_id)
        {
            (
                expr.span,
                format!("{module_name}::{name}"),
                false,
                if path.segments.get(0).is_some_and(|segment| segment.ident.name == module_name) {
                    use_stmts.get(&module_name)
                } else {
                    None
                }
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
                name[..=2].to_ascii_uppercase(),
                true,
                None,
            )
        } else {
            return;
        };

        if !is_from_proc_macro(cx, expr) {
            let msg = if is_method {
                "usage of a legacy numeric method"
            } else {
                "usage of a legacy numeric constant"
            };

            span_lint_and_then(cx, LEGACY_NUMERIC_CONSTANTS, span, msg, |diag| {
                let app = if use_stmts.is_none() {
                    Applicability::MachineApplicable
                } else {
                    Applicability::MaybeIncorrect
                };
                diag.span_suggestion(span, "use the associated constant instead", sugg, app);
                if let Some(use_stmts) = use_stmts {
                    diag.span_note(
                        use_stmts.iter().chain(glob_use_stmts).copied().collect_vec(),
                        "you may need to remove one of the following `use` statements",
                    );
                }
            });
        }
    }
}

fn is_path_in_integral_module(cx: &LateContext<'_>, def_id: DefId) -> Option<Symbol> {
    let path = cx.get_def_path(def_id);
    if let [
        sym::core | sym::std,
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
    ] = &*cx.get_def_path(def_id)
        // So `use` statements like `std::f32` also work
        && path.len() <= 3
    {
        return Some(*module);
    }

    None
}
