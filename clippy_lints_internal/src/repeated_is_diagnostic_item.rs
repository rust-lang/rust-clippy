use crate::internal_paths::MAYBE_DEF;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::print::with_forced_trimmed_paths;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_tool_lint! {
    /// ### What it does
    /// Checks for repeated use of `MaybeDef::is_diag_item`/`TyCtxt::is_diagnostic_item`;
    /// suggests to first call `MaybDef::opt_diag_name`/`TyCtxt::get_diagnostic_name` and then
    /// compare the output with all the `Symbol`s.
    ///
    /// ### Why is this bad?
    /// Each of such calls ultimately invokes the `diagnostic_items` query.
    /// While the query is cached, it's still better to avoid calling it multiple times if possible.
    ///
    /// ### Example
    /// ```no_run
    /// ty.is_diag_item(cx, sym::Option) || ty.is_diag_item(cx, sym::Result)
    /// cx.tcx.is_diagnostic_item(sym::Option, did) || cx.tcx.is_diagnostic_item(sym::Result, did)
    /// ```
    /// Use instead:
    /// ```no_run
    /// matches!(ty.opt_diag_name(cx), Some(sym::Option | sym::Result))
    /// matches!(cx.tcx.get_diagnostic_name(did), Some(sym::Option | sym::Result))
    /// ```
    pub clippy::REPEATED_IS_DIAGNOSTIC_ITEM,
    Warn,
    "repeated use of `MaybeDef::is_diag_item`/`TyCtxt::is_diagnostic_item`"
}
declare_lint_pass!(RepeatedIsDiagnosticItem => [REPEATED_IS_DIAGNOSTIC_ITEM]);

impl LateLintPass<'_> for RepeatedIsDiagnosticItem {
    #[expect(clippy::too_many_lines)]
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::Binary(op, left, right) = expr.kind {
            if op.node == BinOpKind::Or {
                // recv1.is_diag_item(cx, sym1) || recv2.is_diag_item(cx, sym2)
                if let Some((cx1, recv1, sym1)) = extract_is_diag_item(cx, left)
                    && let Some((cx2, recv2, sym2)) = extract_is_diag_item(cx, right)
                    && eq_expr_value(cx, cx1, cx2)
                    && eq_expr_value(cx, recv1, recv2)
                {
                    let recv_ty = with_forced_trimmed_paths!(format!(
                        "{}",
                        cx.typeck_results().expr_ty_adjusted(recv1).peel_refs()
                    ));
                    let recv_ty = recv_ty.trim_end_matches("<'_>");
                    span_lint_and_then(
                        cx,
                        REPEATED_IS_DIAGNOSTIC_ITEM,
                        expr.span,
                        format!("repeated calls to `{recv_ty}::is_diag_item`"),
                        |diag| {
                            diag.note("this calls `TyCtxt::is_diagnostic_item` internally, which is expensive");

                            let mut app = Applicability::MachineApplicable;
                            let cx_str = snippet_with_applicability(cx, cx1.span, "_", &mut app);
                            let recv = snippet_with_applicability(cx, recv1.span, "_", &mut app);
                            let sym1 = snippet_with_applicability(cx, sym1.span, "_", &mut app);
                            let sym2 = snippet_with_applicability(cx, sym2.span, "_", &mut app);
                            diag.span_suggestion_verbose(
                                expr.span,
                                format!("call `{recv_ty}::opt_diag_name`, and reuse the results"),
                                format!("matches!({recv}.opt_diag_name({cx_str}), Some({sym1} | {sym2}))"),
                                app,
                            );
                        },
                    );
                    return;
                }

                // cx.tcx.is_diagnostic_item(sym1, did) || cx.tcx.is_diagnostic_item(sym2, did)
                if let Some((tcx1, recv1, sym1)) = extract_is_diagnostic_item(cx, left)
                    && let Some((tcx2, recv2, sym2)) = extract_is_diagnostic_item(cx, right)
                    && eq_expr_value(cx, tcx1, tcx2)
                    && eq_expr_value(cx, recv1, recv2)
                {
                    span_lint_and_then(
                        cx,
                        REPEATED_IS_DIAGNOSTIC_ITEM,
                        expr.span,
                        "repeated calls to `TyCtxt::is_diagnostic_item`",
                        |diag| {
                            diag.note("this calls an expensive compiler query");

                            let mut app = Applicability::MachineApplicable;
                            let tcx = snippet_with_applicability(cx, tcx1.span, "_", &mut app);
                            let recv = snippet_with_applicability(cx, recv1.span, "_", &mut app);
                            let sym1 = snippet_with_applicability(cx, sym1.span, "_", &mut app);
                            let sym2 = snippet_with_applicability(cx, sym2.span, "_", &mut app);
                            diag.span_suggestion_verbose(
                                expr.span,
                                "call `TyCtxt::get_diagnostic_name`, and reuse the results",
                                format!("matches!({tcx}.get_diagnostic_name({recv}), Some({sym1} | {sym2}))"),
                                app,
                            );
                        },
                    );
                    return;
                }
            }

            if op.node == BinOpKind::And
                && let ExprKind::Unary(UnOp::Not, left) = left.kind
                && let ExprKind::Unary(UnOp::Not, right) = right.kind
            {
                // !recv1.is_diag_item(cx, sym1) && !recv2.is_diag_item(cx, sym2)
                if let Some((cx1, recv1, sym1)) = extract_is_diag_item(cx, left)
                    && let Some((cx2, recv2, sym2)) = extract_is_diag_item(cx, right)
                    && eq_expr_value(cx, cx1, cx2)
                    && eq_expr_value(cx, recv1, recv2)
                {
                    let recv_ty = with_forced_trimmed_paths!(format!(
                        "{}",
                        cx.typeck_results().expr_ty_adjusted(recv1).peel_refs()
                    ));
                    let recv_ty = recv_ty.trim_end_matches("<'_>");
                    span_lint_and_then(
                        cx,
                        REPEATED_IS_DIAGNOSTIC_ITEM,
                        expr.span,
                        format!("repeated calls to `{recv_ty}::is_diag_item`"),
                        |diag| {
                            diag.note("this calls `TyCtxt::is_diagnostic_item` internally, which is expensive");

                            let mut app = Applicability::MachineApplicable;
                            let cx_str = snippet_with_applicability(cx, cx1.span, "_", &mut app);
                            let recv = snippet_with_applicability(cx, recv1.span, "_", &mut app);
                            let sym1 = snippet_with_applicability(cx, sym1.span, "_", &mut app);
                            let sym2 = snippet_with_applicability(cx, sym2.span, "_", &mut app);
                            diag.span_suggestion_verbose(
                                expr.span,
                                format!("call `{recv_ty}::opt_diag_name`, and reuse the results"),
                                format!("!matches!({recv}.opt_diag_name({cx_str}), Some({sym1} | {sym2}))"),
                                app,
                            );
                        },
                    );
                    return;
                }

                // !cx.tcx.is_diagnostic_item(sym1, did) && !cx.tcx.is_diagnostic_item(sym2, did)
                if let Some((tcx1, recv1, sym1)) = extract_is_diagnostic_item(cx, left)
                    && let Some((tcx2, recv2, sym2)) = extract_is_diagnostic_item(cx, right)
                    && eq_expr_value(cx, tcx1, tcx2)
                    && eq_expr_value(cx, recv1, recv2)
                {
                    span_lint_and_then(
                        cx,
                        REPEATED_IS_DIAGNOSTIC_ITEM,
                        expr.span,
                        "repeated calls to `TyCtxt::is_diagnostic_item`",
                        |diag| {
                            diag.note("this calls an expensive compiler query");

                            let mut app = Applicability::MachineApplicable;
                            let tcx = snippet_with_applicability(cx, tcx1.span, "_", &mut app);
                            let recv = snippet_with_applicability(cx, recv1.span, "_", &mut app);
                            let sym1 = snippet_with_applicability(cx, sym1.span, "_", &mut app);
                            let sym2 = snippet_with_applicability(cx, sym2.span, "_", &mut app);
                            diag.span_suggestion_verbose(
                                expr.span,
                                "call `TyCtxt::get_diagnostic_name`, and reuse the results",
                                format!("!matches!({tcx}.get_diagnostic_name({recv}), Some({sym1} | {sym2}))"),
                                app,
                            );
                        },
                    );
                    #[expect(clippy::needless_return, reason = "would become needed if the lint gets expanded")]
                    return;
                }
            }
        }
    }
}

fn extract_is_diag_item<'tcx>(
    cx: &LateContext<'_>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::MethodCall(is_diag_item, recv, [cx_, sym], _) = expr.kind
        && is_diag_item.ident.name == sym::is_diag_item
        // Whether this a method from the `MaybeDef` trait
        && let Some(did) = cx.ty_based_def(expr).opt_parent(cx).opt_def_id()
        && MAYBE_DEF.matches(cx, did)
    {
        Some((cx_, recv, sym))
    } else {
        None
    }
}

fn extract_is_diagnostic_item<'tcx>(
    cx: &LateContext<'_>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::MethodCall(is_diag_item, tcx, [sym, did], _) = expr.kind
        && is_diag_item.ident.name == sym::is_diagnostic_item
        // Whether this is an inherent method on `TyCtxt`
        && cx
            .ty_based_def(expr)
            .opt_parent(cx)
            .opt_impl_ty(cx)
            .is_diag_item(cx, sym::TyCtxt)
    {
        Some((tcx, did, sym))
    } else {
        None
    }
}
