use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet;
use clippy_utils::{msrvs, path_to_local_with_projections, sym};
use rustc_hir::{ClosureKind, Expr, ExprKind, PatKind, PathSegment};
use rustc_lint::{Applicability, LateContext, LateLintPass, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// It lints manual implementation of trim-like methods, which use
    /// `char::is_ascii_whitespace` / `char::is_whitespace` check in closures passed as a parameter.
    /// ### Why is this bad?
    /// It makes code harder to read, and implements functionality, which is already present in stdlib.
    /// ### Example
    /// ```no_run,rust
    /// let _ = "hello".trim_end_matches(|c: char| c.is_ascii_whitespace());
    /// let _ = "bye".trim_matches(|c: char| c.is_whitespace());
    /// ```
    /// Use instead:
    /// ```no_run,rust
    /// let _ = "hello".trim_ascii_end();
    /// let _ = "bye".trim();
    /// ```
    #[clippy::version = "1.102.0"]
    pub MANUAL_TRIM_WHITESPACES,
    complexity,
    "manual implementation of trim methods"
}

impl_lint_pass!(ManualTrimWhitespaces => [MANUAL_TRIM_WHITESPACES]);

pub struct ManualTrimWhitespaces {
    msrv: Msrv,
}

impl ManualTrimWhitespaces {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualTrimWhitespaces {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::MethodCall(seg, recv, [arg], _) = expr.kind
            && let Some(match_type) = extract_trim_method(seg)
            && cx.typeck_results().expr_ty_adjusted(recv).peel_refs().is_str()
            && let Some(pred_type) = extract_predicate_from_closure_expr(cx, arg)
        {
            if matches!(pred_type, PredicateType::Ascii) && !self.msrv.meets(cx, msrvs::STR_TRIM_ASCII) {
                return;
            }

            let method_name = match (pred_type, match_type) {
                (PredicateType::Plain, TrimPosition::Start) => "trim_start",
                (PredicateType::Plain, TrimPosition::End) => "trim_end",
                (PredicateType::Plain, TrimPosition::Both) => "trim",
                (PredicateType::Ascii, TrimPosition::Start) => "trim_ascii_start",
                (PredicateType::Ascii, TrimPosition::End) => "trim_ascii_end",
                (PredicateType::Ascii, TrimPosition::Both) => "trim_ascii",
            };
            let reciever = snippet(cx, recv.span, "..");
            let sugg = format!("{reciever}.{method_name}()");
            span_lint_and_sugg(
                cx,
                MANUAL_TRIM_WHITESPACES,
                expr.span,
                format!("manual implementation of `{method_name}` method"),
                "use",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

enum PredicateType {
    Plain,
    Ascii,
}

fn extract_predicate_from_closure_expr(cx: &LateContext<'_>, expr: &'_ Expr<'_>) -> Option<PredicateType> {
    if let ExprKind::Closure(closure) = expr.kind
        && closure.kind == ClosureKind::Closure
        && let body = cx.tcx.hir_body(closure.body)
        && let [param] = body.params
        && let PatKind::Binding(_, param_hir_id, _, _) = param.pat.kind
        && let ExprKind::MethodCall(method_seg, closure_reciever, [], _) = body.value.kind
        && let Some(expr_reciever_hir_id) = path_to_local_with_projections(closure_reciever)
        && param_hir_id == expr_reciever_hir_id
    {
        return match method_seg.ident.name {
            sym::is_ascii_whitespace => Some(PredicateType::Ascii),
            sym::is_whitespace => Some(PredicateType::Plain),
            _ => None,
        };
    }
    None
}

enum TrimPosition {
    Start,
    End,
    Both,
}

fn extract_trim_method(seg: &PathSegment<'_>) -> Option<TrimPosition> {
    match seg.ident.name {
        sym::trim_matches => Some(TrimPosition::Both),
        sym::trim_start_matches => Some(TrimPosition::Start),
        sym::trim_end_matches => Some(TrimPosition::End),
        _ => None,
    }
}
