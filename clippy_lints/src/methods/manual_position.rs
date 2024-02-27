use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use clippy_utils::{is_diag_item_method, is_diag_trait_item, path_to_local_id, peel_blocks_with_stmt};
use rustc_errors::Applicability;
use rustc_hir::{Body, ClosureKind, Expr, ExprKind, HirId, LangItem, Node, Pat, PatKind, QPath};
use rustc_lint::LateContext;
use rustc_span::{sym, Span};

use super::MANUAL_POSITION;

#[derive(Debug, PartialEq, Eq)]
enum UsageKind {
    Expect(Span),
    Unwrap,
    QuestionMark,
    Map,
}

#[derive(Debug, PartialEq, Eq)]
struct Usage {
    kind: UsageKind,
    end_span: Span,
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, arg: &Expr<'_>, start_span: Span, rev: bool) {
    if let ExprKind::Closure(c) = arg.kind
        && matches!(c.kind, ClosureKind::Closure)
        && let typeck = cx.typeck_results()
        && let Some(fn_id) = typeck.type_dependent_def_id(expr.hir_id)
        && (is_diag_trait_item(cx, fn_id, sym::Iterator))
        && let body = cx.tcx.hir().body(c.body)
        && let [param] = body.params
        && let parent = cx.tcx.hir().parent_iter(expr.hir_id)
        && let Some(usage) = parse_usage(cx, parent)
        && let pat = match param.pat.kind {
            PatKind::Ref(pat, _) => pat,
            _ => param.pat,
        }
        && let PatKind::Tuple([position_arg, item_arg], _) = pat.kind
        && matches!(position_arg.kind, PatKind::Wild)
        && let Some(param_snippet) = snippet_opt(cx, item_arg.span)
        && let Some(predicate_body) = snippet_opt(cx, body.value.span)
        && let Some(usage_sugg) = usage.kind.to_sugg(cx)
    {
        let applicability = Applicability::MaybeIncorrect;
        let rev = if rev { "r" } else { "" };
        let msg = format!("manual implementation of {rev}position");
        span_lint_and_sugg(
            cx,
            MANUAL_POSITION,
            start_span.to(usage.end_span),
            &msg,
            "replace with",
            format!("{rev}position(|{param_snippet}|{predicate_body}){usage_sugg}"),
            applicability,
        );
    }
}

fn parse_usage<'tcx>(cx: &LateContext<'tcx>, mut iter: impl Iterator<Item = (HirId, Node<'tcx>)>) -> Option<Usage> {
    let (kind, end_span) = if let Some((_, Node::Expr(e))) = iter.next() {
        match e.kind {
            ExprKind::Call(
                Expr {
                    kind: ExprKind::Path(QPath::LangItem(LangItem::TryTraitBranch, ..)),
                    ..
                },
                _,
            ) => {
                if let Some((_, Node::Expr(e))) = iter.nth(1)
                    && let Some(span) = is_using_position(e)
                {
                    (UsageKind::QuestionMark, span)
                } else {
                    return None;
                }
            },
            ExprKind::MethodCall(name, _, args, span) if name.ident.name == sym::map => {
                if let Some(map_arg) = args.first()
                    && let ExprKind::Closure(c) = map_arg.kind
                    && matches!(c.kind, ClosureKind::Closure)
                    && let body = cx.tcx.hir().body(c.body)
                    && is_expr_returning_first_field(body)
                {
                    (UsageKind::Map, span)
                } else {
                    return None;
                }
            },
            ExprKind::MethodCall(name, _, [], _)
                if name.ident.name == sym::unwrap
                    && cx
                        .typeck_results()
                        .type_dependent_def_id(e.hir_id)
                        .map_or(false, |id| is_diag_item_method(cx, id, sym::Option)) =>
            {
                if let Some((_, Node::Expr(e))) = iter.next()
                    && let Some(span) = is_using_position(e)
                {
                    (UsageKind::Unwrap, span)
                } else {
                    return None;
                }
            },
            ExprKind::MethodCall(name, _, [param], _)
                if name.ident.name == sym::expect
                    && cx
                        .typeck_results()
                        .type_dependent_def_id(e.hir_id)
                        .map_or(false, |id| is_diag_item_method(cx, id, sym::Option)) =>
            {
                if let Some((_, Node::Expr(e))) = iter.next()
                    && let Some(span) = is_using_position(e)
                {
                    (UsageKind::Expect(param.span), span)
                } else {
                    return None;
                }
            },
            _ => return None,
        }
    } else {
        return None;
    };
    Some(Usage { kind, end_span })
}

impl UsageKind {
    fn to_sugg(&self, cx: &LateContext<'_>) -> Option<String> {
        match self {
            UsageKind::Expect(span) => snippet_opt(cx, *span).map(|span| format!(".expect({span})")),
            UsageKind::Unwrap => Some(".unwrap()".into()),
            UsageKind::QuestionMark => Some("?".into()),
            UsageKind::Map => Some(String::default()),
        }
    }
}

fn is_using_position(expr: &Expr<'_>) -> Option<Span> {
    match expr.kind {
        ExprKind::Field(_, id) if id.name == sym!(0) => Some(expr.span.shrink_to_hi()),
        _ => None,
    }
}

fn is_expr_returning_first_field(func: &Body<'_>) -> bool {
    fn check_pat(pat: &Pat<'_>, expr: &Expr<'_>) -> bool {
        match (&pat.kind, expr.kind) {
            (&PatKind::Binding(_, id, _, _), ExprKind::Field(expr, field)) if field.name == sym!(0) => {
                path_to_local_id(expr, id)
            },
            (PatKind::Tuple([a, _], etc), _) if etc.as_opt_usize().is_none() => {
                if let PatKind::Binding(_, id, _, _) = a.kind {
                    path_to_local_id(expr, id)
                } else {
                    false
                }
            },
            (PatKind::Tuple([a], etc), _) if etc.as_opt_usize().is_some_and(|dot_dot_pos| dot_dot_pos == 1) => {
                if let PatKind::Binding(_, id, _, _) = a.kind {
                    path_to_local_id(expr, id)
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    let [param] = func.params else {
        return false;
    };
    let mut expr = func.value;
    loop {
        expr = peel_blocks_with_stmt(expr);
        match expr.kind {
            ExprKind::Ret(Some(e)) => expr = e,
            _ => return check_pat(param.pat, expr),
        }
    }
}
