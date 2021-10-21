use clippy_utils::{
    diagnostics::span_lint_and_sugg, higher::AssertExpn, is_direct_expn_of, source, ty::implements_trait,
};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Lit};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::Ident;

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about boolean comparisons in assert-like macros.
    ///
    /// ### Why is this bad?
    /// It is shorter to use the equivalent.
    ///
    /// ### Example
    /// ```rust
    /// // Bad
    /// assert_eq!("a".is_empty(), false);
    /// assert_ne!("a".is_empty(), true);
    ///
    /// // Good
    /// assert!(!"a".is_empty());
    /// ```
    pub BOOL_ASSERT_COMPARISON,
    style,
    "Using a boolean as comparison value in an assert_* macro when there is no need"
}

declare_lint_pass!(BoolAssertComparison => [BOOL_ASSERT_COMPARISON]);

fn bool_lit(e: &Expr<'_>) -> Option<bool> {
    match e.kind {
        ExprKind::Lit(Lit {
            node: LitKind::Bool(b), ..
        }) => {
            if e.span.from_expansion() {
                None
            } else {
                Some(b)
            }
        },
        _ => None,
    }
}

fn is_impl_not_trait_with_bool_out(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(e);

    cx.tcx
        .lang_items()
        .not_trait()
        .filter(|trait_id| implements_trait(cx, ty, *trait_id, &[]))
        .and_then(|trait_id| {
            cx.tcx.associated_items(trait_id).find_by_name_and_kind(
                cx.tcx,
                Ident::from_str("Output"),
                ty::AssocKind::Type,
                trait_id,
            )
        })
        .map_or(false, |assoc_item| {
            let proj = cx.tcx.mk_projection(assoc_item.def_id, cx.tcx.mk_substs_trait(ty, &[]));
            let nty = cx.tcx.normalize_erasing_regions(cx.param_env, proj);

            nty.is_bool()
        })
}

impl<'tcx> LateLintPass<'tcx> for BoolAssertComparison {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let macros = ["assert_eq", "debug_assert_eq"];
        let inverted_macros = ["assert_ne", "debug_assert_ne"];

        for (mac, is_eq) in macros
            .iter()
            .map(|el| (el, true))
            .chain(inverted_macros.iter().map(|el| (el, false)))
        {
            if let Some(span) = is_direct_expn_of(expr.span, mac) {
                if let Some(parse_assert) = AssertExpn::parse(expr) {
                    if let [a, b] = parse_assert.assert_arguments()[..] {
                        let (lit_value, other_expr) = match (bool_lit(a), bool_lit(b)) {
                            (Some(lit), None) => (lit, b),
                            (None, Some(lit)) => (lit, a),
                            _ => {
                                // If there are two boolean arguments, we definitely don't understand
                                // what's going on, so better leave things as is...
                                //
                                // Or there is simply no boolean and then we can leave things as is!
                                return;
                            },
                        };

                        if !is_impl_not_trait_with_bool_out(cx, a) || !is_impl_not_trait_with_bool_out(cx, b) {
                            // At this point the expression which is not a boolean
                            // literal does not implement Not trait with a bool output,
                            // so we cannot suggest to rewrite our code
                            return;
                        }

                        let non_eq_mac = &mac[..mac.len() - 3];
                        let mut applicability = Applicability::MachineApplicable;
                        let expr_string = if lit_value ^ is_eq {
                            format!("!({})", source::snippet(cx, other_expr.span, ""))
                        } else {
                            source::snippet(cx, other_expr.span, "").to_string()
                        };
                        let fmt_args = parse_assert.format_arguments(cx, &mut applicability);
                        let arg_span = match &fmt_args[..] {
                            [] => None,
                            [a] => Some(a.to_string()),
                            _ => {
                                let mut args = fmt_args[0].to_string();
                                for el in &fmt_args[1..] {
                                    args.push_str(&format!(", {}", el));
                                }
                                Some(args)
                            },
                        };
                        let suggestion = if let Some(spans) = arg_span {
                            format!("{}!({}, {})", non_eq_mac, expr_string, spans)
                        } else {
                            format!("{}!({})", non_eq_mac, expr_string)
                        };
                        span_lint_and_sugg(
                            cx,
                            BOOL_ASSERT_COMPARISON,
                            span,
                            &format!("used `{}!` with a literal bool", mac),
                            "replace it with",
                            suggestion,
                            applicability,
                        );
                        return;
                    }
                }
            }
        }
    }
}
