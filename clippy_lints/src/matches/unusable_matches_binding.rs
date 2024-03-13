use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_then};
use clippy_utils::visitors::is_local_used;
use rustc_hir::{Arm, PatKind};
use rustc_lint::LateContext;

use super::UNUSABLE_MATCHES_BINDING;

pub(crate) fn check_matches<'tcx>(cx: &LateContext<'tcx>, arms: &'tcx [Arm<'tcx>]) {
    for arm in arms {
        if let PatKind::Binding(_, id, _, None) = arm.pat.kind {
            if !is_local_used(cx, arm.body, id) {
                if let Some(guard) = arm.guard {
                    span_lint_and_help(
                        cx,
                        UNUSABLE_MATCHES_BINDING,
                        guard.body().span,
                        "identifier pattern in `matches!` macro always evaluates to the value of the guard",
                        None,
                        "if you meant to check predicate, then try changing `matches!` macro into predicate the guard's checking",
                    );
                } else {
                    span_lint_and_then(
                        cx,
                        UNUSABLE_MATCHES_BINDING,
                        arm.pat.span,
                        "identifier pattern in `matches!` macro always evaluates to true",
                        |diag| {
                            diag.note("the identifier pattern matches any value and creates an unusable binding in the process")
                                .help("if you meant to compare two values, use `x == y` or `discriminant(x) == discriminant(y)`");
                        },
                    );
                }
            }
        }
    }
}
