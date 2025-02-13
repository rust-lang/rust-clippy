use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_wild;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{indent_of, snippet, snippet_with_applicability};
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use rustc_errors::Applicability;
use rustc_hir::{Arm, Expr, ExprKind, LangItem};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::MATCH_ON_VEC_ITEMS;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, scrutinee: &'tcx Expr<'_>, arms: &'tcx [Arm<'_>], msrv: &Msrv) {
    if let Some(idx_expr) = is_vec_indexing(cx, scrutinee)
        && let ExprKind::Index(vec, idx, _) = idx_expr.kind
    {
        span_lint_and_then(
            cx,
            MATCH_ON_VEC_ITEMS,
            scrutinee.span,
            "indexing into a vector may panic",
            |diag| {
                if msrv.meets(msrvs::OR_PATTERNS) {
                    let mut app = Applicability::MachineApplicable;
                    let mut suggestions: Vec<(Span, String)> = vec![(
                        scrutinee.span,
                        format!(
                            "{}.get({})",
                            snippet_with_applicability(cx, vec.span, "_", &mut app),
                            snippet_with_applicability(cx, idx.span, "_", &mut app)
                        ),
                    )];

                    let mut is_wildcard_exist = false;
                    for arm in arms {
                        let pat = arm.pat;
                        if is_wild(pat) {
                            is_wildcard_exist = true;
                            continue;
                        }
                        let pat_str = snippet_with_applicability(cx, pat.span, "_", &mut app);
                        let suggestion = format!("Some({pat_str})");
                        suggestions.push((pat.span, suggestion));
                    }

                    if !is_wildcard_exist && let Some(last_arm) = arms.last() {
                        let indent_variant = indent_of(cx, last_arm.span).unwrap_or(0);
                        suggestions.push((
                            last_arm.span.shrink_to_hi(),
                            format!("\n{}None => {{}}", " ".repeat(indent_variant)).to_string(),
                        ));
                    }

                    diag.multipart_suggestion("try", suggestions, app);
                } else {
                    diag.span_suggestion(
                        scrutinee.span,
                        "try",
                        format!("{}.get({})", snippet(cx, vec.span, ".."), snippet(cx, idx.span, "..")),
                        Applicability::Unspecified,
                    );
                }
            },
        );
    }
}

fn is_vec_indexing<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Index(array, index, _) = expr.kind
        && is_vector(cx, array)
        && !is_full_range(cx, index)
    {
        return Some(expr);
    }

    None
}

fn is_vector(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    let ty = ty.peel_refs();
    is_type_diagnostic_item(cx, ty, sym::Vec)
}

fn is_full_range(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    let ty = ty.peel_refs();
    is_type_lang_item(cx, ty, LangItem::RangeFull)
}
