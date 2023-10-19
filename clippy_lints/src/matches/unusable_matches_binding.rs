use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::source::snippet_block;
use clippy_utils::visitors::is_local_used;
use rustc_hir::{Arm, Expr, PatKind};
use rustc_lint::LateContext;

use super::UNUSABLE_MATCHES_BINDING;

pub(crate) fn check_matches<'tcx>(cx: &LateContext<'tcx>, ex: &Expr<'tcx>, arms: &'tcx [Arm<'tcx>], expr: &Expr<'tcx>) {
    for arm in arms {
        if let PatKind::Binding(_, id, ident, None) = arm.pat.kind {
            let is_used_in_guard = arm.guard.is_some_and(|guard| is_local_used(cx, guard.body(), id));

            if !is_local_used(cx, arm.body, id) {
                if let Some(guard) = arm.guard {
                    if is_used_in_guard {
                        let first_matches_argument = snippet_block(cx, ex.span, "..", None);
                        span_lint_and_note(
                            cx,
                            UNUSABLE_MATCHES_BINDING,
                            arm.pat.span,
                            &format!(
                                "using identificator (`{}`) as matches! pattern argument matches all cases",
                                ident.name
                            ),
                            Some(expr.span),
                            &format!(
                                "matches! invocation always evaluates to guard where `{}` was replaced by `{}`",
                                ident.name, first_matches_argument
                            ),
                        );
                    } else {
                        span_lint_and_note(
                            cx,
                            UNUSABLE_MATCHES_BINDING,
                            arm.pat.span,
                            &format!(
                                "using identificator (`{}`) as matches! pattern argument without usage in guard has the same effect as evaluating guard",
                                ident.name
                            ),
                            Some(guard.body().span),
                            "try replacing matches! expression with the content of the guard's body",
                        );
                    }
                } else {
                    span_lint_and_note(
                        cx,
                        UNUSABLE_MATCHES_BINDING,
                        arm.pat.span,
                        &format!(
                            "using identificator (`{}`) as matches! pattern argument without guard matches all cases",
                            ident.name
                        ),
                        Some(expr.span),
                        "matches! invocation always evaluates to `true`",
                    );
                }
            }
        }
    }
}
