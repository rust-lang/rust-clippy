use clippy_utils::{diagnostics::span_lint_and_sugg, is_from_proc_macro, is_lang_item_or_ctor, last_path_segment};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{
    lint::in_external_macro,
    ty::{self, Ty},
};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::kw;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for types constructed by `default` that really don't need to be.
    ///
    /// ### Why is this bad?
    /// It's harder for the reader to know what the value is, and it's an unnecessary function call.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a = A(Option::default());
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let a = A(None);
    /// ```
    #[clippy::version = "1.72.0"]
    pub TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
    pedantic,
    "checks for usage of `Default::default` to construct trivial types"
}
declare_lint_pass!(TrivialDefaultConstructedTypes => [TRIVIAL_DEFAULT_CONSTRUCTED_TYPES]);

impl<'tcx> LateLintPass<'tcx> for TrivialDefaultConstructedTypes {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !in_external_macro(cx.sess(), expr.span)
            && let ExprKind::Call(call, _) = expr.kind
            && let ExprKind::Path(qpath) = call.kind
            // `last_path_segment` ICEs if we give it a `LangItem`.
            && !matches!(qpath, QPath::LangItem(..))
            && last_path_segment(&qpath).ident.name == kw::Default
        {
            let ret_ty = cx
                .typeck_results()
                .expr_ty(call)
                .fn_sig(cx.tcx)
                .output()
                .skip_binder()
                .peel_refs();
            if let Some(default) = default_value(cx, ret_ty) && !is_from_proc_macro(cx, expr) {
                span_lint_and_sugg(
                    cx,
                    TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
                    expr.span,
                    "constructing a trivial type using `default`",
                    "try",
                    default.to_string(),
                    Applicability::MachineApplicable,
                );
            } else if let ty::Tuple(fields) = ret_ty.kind()
                && let Some(fields_default) = fields.iter()
                    .map(|field| default_value(cx, field))
                    .collect::<Option<Vec<&'static str>>>()
                && !is_from_proc_macro(cx, expr)
            {
                let default = if fields.len() == 1 {
                    // Needs trailing comma to be a single-element tuple
                    fields_default[0].to_owned() + ","
                } else {
                    fields_default.join(", ")
                };

                span_lint_and_sugg(
                    cx,
                    TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
                    expr.span,
                    &format!(
                        "constructing a {} using `default`",
                        if fields.is_empty() { "unit" } else { "trivial tuple" },
                    ),
                    "try",
                    format!("({default})"),
                    Applicability::MachineApplicable,
                );
            } else if let ty::Array(ty, len) = ret_ty.kind()
                && let Some(default) = default_value(cx, *ty)
                && !is_from_proc_macro(cx, expr)
            {
                span_lint_and_sugg(
                    cx,
                    TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
                    expr.span,
                    "constructing a trivial array using `default`",
                    "try",
                    format!("[{default}; {len}]"),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}

/// Gets the default value of `ty`.
fn default_value(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(def, _) => {
            if is_lang_item_or_ctor(cx, def.did(), LangItem::Option) {
                return Some("None");
            }

            None
        },
        ty::Bool => Some("false"),
        ty::Str => Some(r#""""#),
        ty::Int(_) | ty::Uint(_) => Some("0"),
        ty::Float(_) => Some("0.0"),
        // Do not handle `ty::Char`, it's a lot less readable
        _ => None,
    }
}
