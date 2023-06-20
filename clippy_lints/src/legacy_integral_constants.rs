use clippy_utils::{
    diagnostics::span_lint_and_then,
    get_parent_expr, is_from_proc_macro, last_path_segment,
    msrvs::{self, Msrv},
    std_or_core,
};
use rustc_errors::Applicability;
use rustc_hir::{def::Res, def_id::DefId};
use rustc_hir::{Expr, ExprKind, PrimTy, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Symbol};

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
    pub LEGACY_INTEGRAL_CONSTANTS,
    style,
    "checks for usage of legacy std integral constants"
}
pub struct LegacyIntegralConstants {
    msrv: Msrv,
}

impl LegacyIntegralConstants {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(LegacyIntegralConstants => [LEGACY_INTEGRAL_CONSTANTS]);

impl<'tcx> LateLintPass<'tcx> for LegacyIntegralConstants {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !self.msrv.meets(msrvs::STD_INTEGRAL_CONSTANTS) || in_external_macro(cx.sess(), expr.span) {
            return;
        }
        let ExprKind::Path(qpath) = expr.kind else {
            return;
        };

        // `std::<integer>::<CONST>` check
        let (span, sugg, is_method, needs_reexported_primitive) = if let QPath::Resolved(_, path) = qpath
            && let Some(def_id) = path.res.opt_def_id()
            && let Some(name) = path.segments.iter().last().map(|segment| segment.ident.name)
            && let Some(module_name) = is_path_in_integral_module(cx, def_id)
        {
            (
                expr.span,
                format!("{module_name}::{name}"),
                false,
                path.segments.get(0).is_some_and(|segment| segment.ident.name == module_name)
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
                false,
            )
        } else {
            return;
        };

        if !is_from_proc_macro(cx, expr) {
            let msg = if is_method {
                "usage of a legacy integral constant method"
            } else {
                "usage of a legacy integral constant"
            };

            span_lint_and_then(cx, LEGACY_INTEGRAL_CONSTANTS, span, msg, |diag| {
                // Add `std::primitive` if necessary
                let new_sugg = if needs_reexported_primitive
                    && let Some(std_or_core) = std_or_core(cx)
                {
                    format!("{std_or_core}::primitive::{sugg}")
                } else {
                    sugg.clone()
                };
                diag.span_suggestion(span, "try", new_sugg, Applicability::MachineApplicable);
                // If we added `std::primitive`, also suggest without it
                if needs_reexported_primitive && let Some((module_name, _)) = sugg.split_once("::") {
                    diag.note(format!(
                        "if you remove the `use` statement that introduces `{module_name}`, using the above is \
                         unnecessary",
                    ));
                }
            });
        }
    }

    extract_msrv_attr!(LateContext);
}

fn is_path_in_integral_module(cx: &LateContext<'_>, def_id: DefId) -> Option<Symbol> {
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
        _,
    ] = &*cx.get_def_path(def_id)
    {
        return Some(*module);
    }

    None
}
