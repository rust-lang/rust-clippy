use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{pat_is_wild, sym};
use rustc_hir::{Body, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for iterating a map (`HashMap` or `BTreeMap`) and
    /// ignoring either the keys or values.
    ///
    /// ### Why is this bad?
    ///
    /// Readability. There are `keys` and `values` methods that
    /// can be used to express that we only need the keys or the values.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// # use std::collections::HashMap;
    /// let map: HashMap<u32, u32> = HashMap::new();
    /// let values = map.iter().map(|(_, value)| value).collect::<Vec<_>>();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # use std::collections::HashMap;
    /// let map: HashMap<u32, u32> = HashMap::new();
    /// let values = map.values().collect::<Vec<_>>();
    /// ```
    #[clippy::version = "1.66.0"]
    pub ITER_KV_MAP,
    complexity,
    "iterating on map using `iter` when `keys` or `values` would do"
}

/// lint use of:
///
/// - `hashmap.iter().map(|(_, v)| v)`
/// - `hashmap.into_iter().map(|(_, v)| v)`
///
/// on `HashMaps` and `BTreeMaps` in std
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    map_type: Symbol,        // iter / into_iter
    expr: &'tcx Expr<'tcx>,  // .iter().map(|(_, v_| v))
    recv: &'tcx Expr<'tcx>,  // hashmap
    m_arg: &'tcx Expr<'tcx>, // |(_, v)| v
    msrv: Msrv,
) {
    if map_type == sym::into_iter && !msrv.meets(cx, msrvs::INTO_KEYS) {
        return;
    }
    if !expr.span.from_expansion()
        && let ExprKind::Closure(c) = m_arg.kind
        && let Body {
            params: [p],
            value: body_expr,
        } = cx.tcx.hir_body(c.body)
        && let PatKind::Tuple([key_pat, val_pat], _) = p.pat.kind
        && let (replacement_kind, annotation, bound_ident) = match (&key_pat.kind, &val_pat.kind) {
            (key, PatKind::Binding(ann, _, value, _)) if pat_is_wild(cx, key, m_arg) => ("value", ann, value),
            (PatKind::Binding(ann, _, key, _), value) if pat_is_wild(cx, value, m_arg) => ("key", ann, key),
            _ => return,
        }
        && let ty = cx.typeck_results().expr_ty_adjusted(recv).peel_refs()
        && (is_type_diagnostic_item(cx, ty, sym::HashMap) || is_type_diagnostic_item(cx, ty, sym::BTreeMap))
    {
        let mut applicability = rustc_errors::Applicability::MachineApplicable;
        let recv_snippet = snippet_with_applicability(cx, recv.span, "map", &mut applicability);
        let into_prefix = if map_type == sym::into_iter { "into_" } else { "" };

        if let ExprKind::Path(rustc_hir::QPath::Resolved(_, path)) = body_expr.kind
            && let [local_ident] = path.segments
            && local_ident.ident.name == bound_ident.name
        {
            span_lint_and_sugg(
                cx,
                ITER_KV_MAP,
                expr.span,
                format!("iterating on a map's {replacement_kind}s"),
                "try",
                format!("{recv_snippet}.{into_prefix}{replacement_kind}s()"),
                applicability,
            );
        } else {
            span_lint_and_sugg(
                cx,
                ITER_KV_MAP,
                expr.span,
                format!("iterating on a map's {replacement_kind}s"),
                "try",
                format!(
                    "{recv_snippet}.{into_prefix}{replacement_kind}s().map(|{}{bound_ident}| {})",
                    annotation.prefix_str(),
                    snippet_with_applicability(cx, body_expr.span, "/* body */", &mut applicability)
                ),
                applicability,
            );
        }
    }
}
