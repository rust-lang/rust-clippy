use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeResPath as _};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{is_none_expr, peel_blocks};
use rustc_errors::Applicability;
use rustc_hir::attrs::LangItem;
use rustc_hir::attrs::LangItem::OptionNone;
use rustc_hir::attrs::lang_items::LangItem::ResultErr;
use rustc_hir::{Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::Span;
use rustc_span::symbol::sym;

use super::OBFUSCATED_AND_THEN;

/// lint use of
/// - `res.map_or_else(Err, f)` for `Result`s
/// - `opt.map_or_else(|| None, f)` for `Option`s
///
/// which are both equivalent to `_.and_then(f)`.
pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    recv: &Expr<'_>,
    def_arg: &Expr<'_>,
    map_arg: &Expr<'_>,
    call_span: Span,
) {
    if expr.span.from_expansion() {
        return;
    }

    let ty = cx.typeck_results().expr_ty(recv);
    let msg = if ty.is_diag_item(cx, sym::Result) && is_wrapping_closure(cx, def_arg, ResultErr) {
        "`map_or_else(Err, ..)` on a `Result` can be replaced with `and_then`"
    } else if ty.is_diag_item(cx, sym::Option) && is_wrapping_closure(cx, def_arg, OptionNone) {
        "`map_or_else(|| None, ..)` on an `Option` can be replaced with `and_then`"
    } else {
        return;
    };

    span_lint_and_then(cx, OBFUSCATED_AND_THEN, expr.span, msg, |diag| {
        let mut applicability = Applicability::MachineApplicable;
        let map_snippet = snippet_with_applicability(cx, map_arg.span, "..", &mut applicability);
        diag.span_suggestion_verbose(
            call_span,
            "use `and_then` instead",
            format!("and_then({map_snippet})"),
            applicability,
        );
    });
}

fn is_wrapping_closure(cx: &LateContext<'_>, arg: &Expr<'_>, expected_item: LangItem) -> bool {
    // direct constructor/path: `Err`, `Some`, `None` etc.
    if arg.res(cx).ctor_parent(cx).is_lang_item(cx, expected_item) {
        return true;
    }

    // closure cases
    if let ExprKind::Closure(closure) = arg.kind {
        let body = cx.tcx.hir_body(closure.body);
        let body_val = peel_blocks(body.value);

        // special-case `|| None` when expected_ctor is OptionNone
        if expected_item == OptionNone && body.params.is_empty() && is_none_expr(cx, body_val) {
            return true;
        }

        // single-param closures like `|x| Err(x)`
        if let [param] = body.params
            && let PatKind::Binding(_, param_id, ..) = param.pat.kind
            && let ExprKind::Call(callee, [call_arg]) = body_val.kind
            && callee.res(cx).ctor_parent(cx).is_lang_item(cx, expected_item)
            && call_arg.res_local_id() == Some(param_id)
        {
            return true;
        }
    }

    false
}
