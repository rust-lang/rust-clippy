use crate::manual_ignore_case_cmp::MatchType::{Literal, ToAscii};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{method_chain_args, sym};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::ExprKind::{Binary, Closure, Lit, MethodCall};
use rustc_hir::{BinOpKind, Expr, LangItem, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_middle::ty::{Ty, UintTy};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual case-insensitive ASCII comparison.
    ///
    /// ### Why is this bad?
    /// The `eq_ignore_ascii_case` method is faster because it does not allocate
    /// memory for the new strings, and it is more readable.
    ///
    /// ### Example
    /// ```no_run
    /// fn compare(a: &str, b: &str) -> bool {
    ///     a.to_ascii_lowercase() == b.to_ascii_lowercase() || a.to_ascii_lowercase() == "abc"
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn compare(a: &str, b: &str) -> bool {
    ///     a.eq_ignore_ascii_case(b) || a.eq_ignore_ascii_case("abc")
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub MANUAL_IGNORE_CASE_CMP,
    perf,
    "manual case-insensitive ASCII comparison"
}

declare_lint_pass!(ManualIgnoreCaseCmp => [MANUAL_IGNORE_CASE_CMP]);

enum MatchType<'a> {
    ToAscii(bool, Ty<'a>),
    Literal(LitKind),
}

fn get_ascii_type<'a>(cx: &LateContext<'a>, kind: rustc_hir::ExprKind<'_>) -> Option<(Span, MatchType<'a>)> {
    if let MethodCall(path, expr, _, _) = kind {
        let is_lower = match path.ident.name {
            sym::to_ascii_lowercase => true,
            sym::to_ascii_uppercase => false,
            _ => return None,
        };
        let ty_raw = cx.typeck_results().expr_ty(expr);
        let ty = ty_raw.peel_refs();
        if needs_ref_to_cmp(cx, ty)
            || ty.is_str()
            || ty.is_slice()
            || matches!(ty.opt_diag_name(cx), Some(sym::OsStr | sym::OsString))
        {
            return Some((expr.span, ToAscii(is_lower, ty_raw)));
        }
    } else if let Lit(expr) = kind {
        return Some((expr.span, Literal(expr.node)));
    }
    None
}

struct CharsMap<'tcx> {
    expr: &'tcx Expr<'tcx>,
    is_lower: bool,
}

fn ascii_case_map_closure(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<bool> {
    if let Closure(closure) = expr.kind
        && let body = cx.tcx.hir_body(closure.body)
        && body.params.len() == 1
        && let PatKind::Binding(_, binding, ..) = body.params[0].pat.kind
        && let MethodCall(path, receiver, [], _) = body.value.kind
        && receiver.res_local_id() == Some(binding)
    {
        match path.ident.name {
            sym::to_ascii_lowercase => Some(true),
            sym::to_ascii_uppercase => Some(false),
            _ => None,
        }
    } else {
        None
    }
}

fn ascii_case_mapped_chars<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<CharsMap<'tcx>> {
    if let Some(args) = method_chain_args(expr, &[sym::chars, sym::map])
        && args[0].1.is_empty()
        && let [map_arg] = args[1].1
        && let Some(is_lower) = ascii_case_map_closure(cx, map_arg)
        && cx.typeck_results().expr_ty_adjusted(args[0].0).peel_refs().is_str()
    {
        Some(CharsMap {
            expr: args[0].0,
            is_lower,
        })
    } else {
        None
    }
}

fn get_chars_cmp<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(bool, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let MethodCall(path, cmp_expr, [], _) = expr.kind {
        let is_eq = if path.ident.name == sym::is_eq {
            true
        } else if path.ident.name == sym::is_ne {
            false
        } else {
            return None;
        };

        if let MethodCall(path, left_iter, [right_iter], _) = cmp_expr.kind
            && path.ident.name == sym::cmp
            && let Some(left) = ascii_case_mapped_chars(cx, left_iter)
            && let Some(right) = ascii_case_mapped_chars(cx, right_iter)
            && left.is_lower == right.is_lower
        {
            return Some((is_eq, left.expr, right.expr));
        }
    }

    None
}

fn emit_lint(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    left_span: Span,
    right_span: Span,
    right_val: &MatchType<'_>,
    neg: &str,
) {
    let deref = match right_val {
        ToAscii(_, ty) if needs_ref_to_cmp(cx, *ty) => "&",
        Literal(LitKind::Char(_) | LitKind::Byte(_)) => "&",
        ToAscii(..) | Literal(_) => "",
    };
    span_lint_and_then(
        cx,
        MANUAL_IGNORE_CASE_CMP,
        expr.span,
        "manual case-insensitive ASCII comparison",
        |diag| {
            let mut app = Applicability::MachineApplicable;
            let (left_snip, _) = snippet_with_context(cx, left_span, expr.span.ctxt(), "..", &mut app);
            let (right_snip, _) = snippet_with_context(cx, right_span, expr.span.ctxt(), "..", &mut app);
            diag.span_suggestion_verbose(
                expr.span,
                "consider using `.eq_ignore_ascii_case()` instead",
                format!("{neg}{left_snip}.eq_ignore_ascii_case({deref}{right_snip})"),
                app,
            );
        },
    );
}

fn emit_chars_cmp_lint(cx: &LateContext<'_>, expr: &Expr<'_>, is_eq: bool, left: &Expr<'_>, right: &Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(right);
    let right_val = ToAscii(true, ty);
    let neg = if is_eq { "" } else { "!" };
    emit_lint(cx, expr, left.span, right.span, &right_val, neg);
}

/// Returns true if the type needs to be dereferenced to be compared
fn needs_ref_to_cmp(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    ty.is_char()
        || *ty.kind() == ty::Uint(UintTy::U8)
        || ty.is_diag_item(cx, sym::Vec)
        || ty.is_lang_item(cx, LangItem::String)
}

impl<'tcx> LateLintPass<'tcx> for ManualIgnoreCaseCmp {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // check if expression represents a comparison of two strings
        // using .to_ascii_lowercase() or .to_ascii_uppercase() methods,
        // or one of the sides is a literal
        // Offer to replace it with .eq_ignore_ascii_case() method
        if let Binary(op, left, right) = &expr.kind
            && (op.node == BinOpKind::Eq || op.node == BinOpKind::Ne)
            && let Some((left_span, left_val)) = get_ascii_type(cx, left.kind)
            && let Some((right_span, right_val)) = get_ascii_type(cx, right.kind)
            && match (&left_val, &right_val) {
                (ToAscii(l_lower, ..), ToAscii(r_lower, ..)) if l_lower == r_lower => true,
                (ToAscii(..), Literal(..)) | (Literal(..), ToAscii(..)) => true,
                _ => false,
            }
        {
            let neg = if op.node == BinOpKind::Ne { "!" } else { "" };
            emit_lint(cx, expr, left_span, right_span, &right_val, neg);
        } else if let Some((is_eq, left, right)) = get_chars_cmp(cx, expr) {
            emit_chars_cmp_lint(cx, expr, is_eq, left, right);
        }
    }
}
