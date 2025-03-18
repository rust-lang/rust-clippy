use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, Path, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::Span;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::get_parent_expr;
use clippy_utils::source::snippet_opt;

declare_clippy_lint! {
    /// ### What it does
    /// Suggests to replace `==` and `!=` comparisons with `matches!` macro.
    ///
    /// ### Why is this bad?
    /// It generates smaller assembly.
    ///
    /// ### Example
    /// ```no_run
    /// let d1 = [0, 1];
    /// let d2 = Foo { first: 0, second: 4 };
    /// let d3 = (0, 1);
    ///
    /// d1 == [1, 2];
    /// d2 == Foo { first: 1, second: 2 };
    /// d3 == (1, 2) || d3 == (2, 3) || d3 == (1, 4);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let d1 = [0, 1];
    /// let d2 = Foo { first: 0, second: 4 };
    /// let d3 = (0, 1);
    ///
    /// matches!(d1, [1, 2]);
    /// matches!(d2, Foo { first: 1, second: 2 });
    /// matches!(d3, (1, 2) | (2, 3) | (1, 4));
    /// ```
    #[clippy::version = "1.87.0"]
    pub EQ_SHOULD_BE_MATCH,
    perf,
    "eq comparison should be a pattern matching"
}

impl_lint_pass!(EqShouldBeMatch => [EQ_SHOULD_BE_MATCH]);

#[derive(Default)]
pub(crate) struct EqShouldBeMatch {
    handled_spans: FxHashSet<Span>,
}

fn is_primitive(typ: Ty<'_>) -> bool {
    matches!(
        typ.kind(),
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str
    )
}

fn get_if_literal<'a>(
    cx: &LateContext<'_>,
    left: &'a Expr<'a>,
    right: &'a Expr<'a>,
) -> Option<(&'a Expr<'a>, &'a Expr<'a>)> {
    match (left.kind, right.kind) {
        (_, ExprKind::Tup(_) | ExprKind::Struct(..) | ExprKind::Array(_)) => {
            // All good!
            let typeck = cx.typeck_results();
            if let Some(left_ty) = typeck.expr_ty_opt(left).map(|ty| ty.peel_refs())
                && !is_primitive(left_ty)
            {
                Some((left, right))
            } else {
                None
            }
        },
        (ExprKind::Tup(_) | ExprKind::Struct(..) | ExprKind::Array(_), _) => {
            // We switch args.
            get_if_literal(cx, right, left)
        },
        (_, _) => None,
    }
}

fn emit_lint(cx: &LateContext<'_>, expr_span: Span, left: &Expr<'_>, right: String, op: BinOpKind) {
    if let Some(left_expr) = snippet_opt(cx, left.span) {
        span_lint_and_then(
            cx,
            EQ_SHOULD_BE_MATCH,
            expr_span,
            "this comparison would be faster with pattern matching",
            |diag| {
                diag.span_suggestion(
                    expr_span,
                    "try",
                    format!(
                        "{}matches!({left_expr}, {right})",
                        if op == BinOpKind::Ne { "!" } else { "" },
                    ),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

// Extend the span if the `expr_span` starts sooner or ends later than `span`.
fn extend_span(span: &mut Span, expr_span: Span) {
    if span.lo() > expr_span.lo() {
        *span = span.with_lo(expr_span.lo())
    }
    if span.hi() < expr_span.hi() {
        *span = span.with_hi(expr_span.hi())
    }
}

impl EqShouldBeMatch {
    // This method generates the suggestion to group items if there is more than one,
    // we simply suggest the basic `matches!` replacement.
    fn handle_current(
        &mut self,
        cx: &LateContext<'_>,
        current: &mut Vec<(&Expr<'_>, &Expr<'_>, BinOpKind)>,
        current_info: &mut Option<(&Path<'_>, BinOpKind)>,
    ) {
        if let Some((left, _, op)) = current.first() {
            let mut span = left.span;
            let mut out = String::new();
            let mut should_emit_lint = true;

            for (left, right, _) in current.iter() {
                extend_span(&mut span, left.span);
                extend_span(&mut span, right.span);

                if let Some(snippet) = snippet_opt(cx, right.span) {
                    if !out.is_empty() {
                        out.push_str(" | ");
                    }
                    out.push_str(&snippet);
                } else {
                    should_emit_lint = false;
                    break;
                }
            }
            if should_emit_lint {
                for (left, _, _) in current.iter() {
                    if let Some(parent_expr) = get_parent_expr(cx, left) {
                        self.handled_spans.insert(parent_expr.span);
                    }
                }
                emit_lint(cx, span, left, out, *op);
            }
        }
        current.clear();
        *current_info = None;
    }

    fn handle_or_expr<'a>(
        &self,
        cx: &LateContext<'_>,
        expr: &'a Expr<'a>,
        comparisons: &mut Vec<(&'a Expr<'a>, &'a Expr<'a>, BinOpKind)>,
    ) -> bool {
        if self.handled_spans.contains(&expr.span) {
            return true;
        }
        match expr.kind {
            ExprKind::Binary(bin_op, sub_left, sub_right) => match bin_op.node {
                BinOpKind::Eq | BinOpKind::Ne => {
                    if let Some((sub_left, sub_right)) = get_if_literal(cx, sub_left, sub_right) {
                        comparisons.push((sub_left, sub_right, bin_op.node));
                        true
                    } else {
                        false
                    }
                },
                BinOpKind::Or => {
                    self.handle_or_expr(cx, sub_left, comparisons) && self.handle_or_expr(cx, sub_right, comparisons)
                },
                _ => false,
            },
            _ => false,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for EqShouldBeMatch {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Binary(op, left, right) = expr.kind {
            match op.node {
                BinOpKind::Eq | BinOpKind::Ne if !self.handled_spans.contains(&expr.span) => {
                    if let Some((left, right)) = get_if_literal(cx, left, right)
                        && let Some(right) = snippet_opt(cx, right.span)
                    {
                        emit_lint(cx, expr.span, left, right, op.node)
                    }
                },
                // In case we have `x == (1, 2) || x == (3, 4)`, we go through it and check
                // if it can be simplified. For that, we need to check that `x` is actually the
                // same one.
                //
                // If there is any comparison in between that cannot be grouped with the others,
                // then we render for the group we already have and then we resume.
                BinOpKind::Or if !self.handled_spans.contains(&expr.span) => {
                    let mut comparisons = Vec::new();

                    self.handle_or_expr(cx, left, &mut comparisons);
                    self.handle_or_expr(cx, right, &mut comparisons);
                    if comparisons.len() > 1 {
                        // There might be multiple parts that can be grouped, so let's group
                        // them if possible.
                        let mut current_info = None;
                        let mut current = Vec::new();

                        for (left, right, op) in comparisons.iter() {
                            // If we encounter an item which is not a variable, we render the rest
                            // and skip this one.
                            let ExprKind::Path(QPath::Resolved(_, path)) = left.kind else {
                                self.handle_current(cx, &mut current, &mut current_info);
                                continue;
                            };
                            match current_info {
                                Some((current_path, current_op)) => {
                                    // If the current comparison cannot be grouped with the previous
                                    // one, we render the previous one then resume.
                                    if current_path.res != path.res || current_op != *op {
                                        self.handle_current(cx, &mut current, &mut current_info);
                                    }
                                },
                                None => {},
                            }
                            current.push((left, right, *op));
                            current_info = Some((path, *op));
                        }
                        self.handle_current(cx, &mut current, &mut current_info);
                    }
                },
                _ => {},
            }
        }
    }
}
