use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::expr_or_init;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_slice_like;
use itertools::Itertools;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Pat, PatKind};
use rustc_lint::LateContext;

use super::{CONST_SIZED_CHUNKS_EXACT, CONST_SIZED_CHUNKS_EXACT_MUT, CONST_SIZED_WINDOWS};

/// Checks for the `/CONST_SIZED(_CHUNKS_EXACT(_MUT)?|_WINDOWS)/` lint.
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, pat: &'tcx Pat<'_>, arg: &'tcx Expr<'_>) {
    if !arg.span.from_expansion()
        // The `for` loop pattern should be a binding.
        && let PatKind::Binding(..) = pat.kind
        // The `for` loop argument expression must be a method call.
        && let ExprKind::MethodCall(method, self_arg, [arg], span) = arg.kind
        // The receiver of the method call must be slice-like.
        && is_slice_like(cx, cx.typeck_results().expr_ty(self_arg).peel_refs())
        // The parameter to the method call must be a constant.
        && let Some(Constant::Int(n)) = ConstEvalCtxt::new(cx).eval(expr_or_init(cx, arg))
        // The number of elements should be limited.
        && let Ok(n) = n.try_into() && n >= 1 && n <= 1 + 'z' as usize - 'a' as usize
    {
        let method = method.ident.name;
        let new_method;

        let lint = match method {
            clippy_utils::sym::chunks_exact => {
                new_method = clippy_utils::sym::array_chunks;
                CONST_SIZED_CHUNKS_EXACT
            },
            clippy_utils::sym::chunks_exact_mut => {
                new_method = clippy_utils::sym::array_chunks_mut;
                CONST_SIZED_CHUNKS_EXACT_MUT
            },
            rustc_span::sym::windows => {
                new_method = clippy_utils::sym::array_windows;
                CONST_SIZED_WINDOWS
            },
            _ => return,
        };

        let bindings = ('a'..).take(n).join(", ");
        let self_arg = snippet(cx, self_arg.span, "..");
        let arg = snippet(cx, arg.span, "_");
        let msg = format!("iterating over `{method}()` with constant parameter `{arg}`");

        span_lint_and_then(cx, lint, span, msg, |diag| {
            diag.span_suggestion_verbose(
                span.with_lo(pat.span.lo()),
                format!("use `{new_method}::<{n}>()` instead"),
                format!("[{bindings}] in {self_arg}.{new_method}()"),
                Applicability::Unspecified,
            );
        });
    }
}
