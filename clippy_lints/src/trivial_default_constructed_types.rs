use clippy_utils::{diagnostics::span_lint_and_sugg, is_from_proc_macro, is_lang_item_or_ctor, is_trait_item};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;
use std::borrow::Cow;

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
        if !expr.span.from_expansion()
            && let ExprKind::Call(call, _) = expr.kind
            && is_trait_item(cx, call, sym::Default)
        {
            let mut app = Applicability::MachineApplicable;
            let ret_ty = cx
                .typeck_results()
                .expr_ty(call)
                .fn_sig(cx.tcx)
                .output()
                .skip_binder()
                .peel_refs();
            if let Some(default) = default_value(cx, ret_ty, &mut app) && !is_from_proc_macro(cx, expr) {
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
                && fields.len() <= 3
                && let Some(fields_default) = fields.iter()
                    .map(|field| default_value(cx, field, &mut app))
                    .collect::<Option<Vec<_>>>()
                && !is_from_proc_macro(cx, expr)
            {
                let default = if let [default] = &*fields_default {
                    // Needs trailing comma to be a single-element tuple
                    format!("{default},")
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
                && let Some(default) = default_value(cx, *ty, &mut app)
                && !is_from_proc_macro(cx, expr)
            {
                span_lint_and_sugg(
                    cx,
                    TRIVIAL_DEFAULT_CONSTRUCTED_TYPES,
                    expr.span,
                    "constructing a trivial array using `default`",
                    "try",
                    format!("[{default}; {len}]"),
                    app,
                );
            }
        }
    }
}

/// Gets the default value of `ty`.
fn default_value(cx: &LateContext<'_>, ty: Ty<'_>, app: &mut Applicability) -> Option<Cow<'static, str>> {
    match ty.kind() {
        ty::Adt(def, _) => {
            *app = Applicability::HasPlaceholders;
            // Checking if the generic argument is required would substantially increase the
            // complexity of this lint, for now, just use a placeholder (`_`).
            is_lang_item_or_ctor(cx, def.did(), LangItem::Option).then(|| "None::<_>".into())
        },
        ty::Bool => Some("false".into()),
        ty::Str => Some(r#""""#.into()),
        ty::Int(suffix) => Some(format!("0{}", suffix.name_str()).into()),
        ty::Uint(suffix) => Some(format!("0{}", suffix.name_str()).into()),
        ty::Float(suffix) => Some(format!("0.0{}", suffix.name_str()).into()),
        // Do not handle `ty::Char`, it's a lot less readable
        _ => None,
    }
}
