use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_no_std_crate;
use clippy_utils::sugg::Sugg;
use rustc_ast::BorrowKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node, QPath};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::SWAP_WITH_TEMPORARY;

const MSG_TEMPORARY: &str = "this expression returns a temporary value";

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, func: &Expr<'_>, args: &[Expr<'_>]) {
    if let ExprKind::Path(QPath::Resolved(_, func_path)) = func.kind
        && let Some(func_def_id) = func_path.res.opt_def_id()
        && cx.tcx.is_diagnostic_item(sym::mem_swap, func_def_id)
    {
        match (ArgKind::new(&args[0]), ArgKind::new(&args[1])) {
            (ArgKind::RefMutToTemp(left_temp), ArgKind::RefMutToTemp(right_temp)) => {
                emit_lint_useless(cx, expr, func, &args[0], &args[1], left_temp, right_temp);
            },
            (ArgKind::RefMutToTemp(left_temp), right) => emit_lint_assign(cx, expr, &right, left_temp),
            (left, ArgKind::RefMutToTemp(right_temp)) => emit_lint_assign(cx, expr, &left, right_temp),
            _ => {},
        }
    }
}

enum ArgKind<'tcx> {
    // Mutable reference to a place, coming from a macro
    RefMutToPlaceAsMacro(&'tcx Expr<'tcx>),
    // Place behind a mutable reference
    RefMutToPlace(&'tcx Expr<'tcx>),
    // Temporary value behind a mutable reference
    RefMutToTemp(&'tcx Expr<'tcx>),
    // Any other case
    Expr(&'tcx Expr<'tcx>),
}

impl<'tcx> ArgKind<'tcx> {
    fn new(arg: &'tcx Expr<'tcx>) -> Self {
        if let ExprKind::AddrOf(BorrowKind::Ref, _, target) = arg.kind {
            if target.is_syntactic_place_expr() {
                if arg.span.from_expansion() {
                    ArgKind::RefMutToPlaceAsMacro(arg)
                } else {
                    ArgKind::RefMutToPlace(target)
                }
            } else {
                ArgKind::RefMutToTemp(target)
            }
        } else {
            ArgKind::Expr(arg)
        }
    }
}

fn emit_lint_useless(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    func: &Expr<'_>,
    left: &Expr<'_>,
    right: &Expr<'_>,
    left_temp: &Expr<'_>,
    right_temp: &Expr<'_>,
) {
    span_lint_and_then(
        cx,
        SWAP_WITH_TEMPORARY,
        expr.span,
        "swapping temporary values has no effect",
        |diag| {
            const DROP_MSG: &str = "drop them if creating these temporary expressions is necessary";

            diag.span_note(left_temp.span, MSG_TEMPORARY);
            diag.span_note(right_temp.span, MSG_TEMPORARY);

            // If the `swap()` is a statement by itself, just propose to replace `swap(&mut a, &mut b)` by `a;
            // b;` in order to drop `a` and `b` while acknowledging their side effects. If the
            // `swap()` call is part of a larger expression, replace it by `{core,
            // std}::mem::drop((a, b))`.
            if matches!(cx.tcx.parent_hir_node(expr.hir_id), Node::Stmt(..)) {
                diag.multipart_suggestion(
                    DROP_MSG,
                    vec![
                        (func.span.with_hi(left_temp.span.lo()), String::new()),
                        (left_temp.span.between(right_temp.span), String::from("; ")),
                        (expr.span.with_lo(right_temp.span.hi()), String::new()),
                    ],
                    Applicability::MachineApplicable,
                );
            } else {
                diag.multipart_suggestion(
                    DROP_MSG,
                    vec![
                        (
                            func.span,
                            format!("{}::mem::drop(", if is_no_std_crate(cx) { "core" } else { "std" }),
                        ),
                        (left.span.with_hi(left_temp.span.lo()), String::new()),
                        (right.span.with_hi(right_temp.span.lo()), String::new()),
                        (expr.span.shrink_to_hi(), String::from(")")),
                    ],
                    Applicability::MachineApplicable,
                );
            }
        },
    );
}

fn emit_lint_assign(cx: &LateContext<'_>, expr: &Expr<'_>, target: &ArgKind<'_>, temp: &Expr<'_>) {
    span_lint_and_then(
        cx,
        SWAP_WITH_TEMPORARY,
        expr.span,
        "swapping with a temporary value is inefficient",
        |diag| {
            diag.span_note(temp.span, MSG_TEMPORARY);
            let mut applicability = Applicability::MachineApplicable;
            let ctxt = expr.span.ctxt();
            let assign_target = match target {
                ArgKind::Expr(target) | ArgKind::RefMutToPlaceAsMacro(target) => {
                    Sugg::hir_with_context(cx, target, ctxt, "_", &mut applicability).deref()
                },
                ArgKind::RefMutToPlace(target) => Sugg::hir_with_context(cx, target, ctxt, "_", &mut applicability),
                ArgKind::RefMutToTemp(_) => unreachable!(),
            };
            let assign_source = Sugg::hir_with_context(cx, temp, ctxt, "_", &mut applicability);

            // If the `swap()` is a statement by itself, propose to replace it by `a = b`. Otherwise, when part
            // of a larger expression, surround the assignment with a block to make it `()`.
            let suggestion = format!("{assign_target} = {assign_source}");
            let suggestion = if matches!(cx.tcx.parent_hir_node(expr.hir_id), Node::Stmt(..)) {
                suggestion
            } else {
                format!("{{ {suggestion}; }}")
            };
            diag.span_suggestion(expr.span, "use assignment instead", suggestion, applicability);
        },
    );
}
