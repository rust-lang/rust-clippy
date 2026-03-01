use super::WHILE_LET_LOOP;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{reindent_multiline, snippet, snippet_indent, snippet_opt, snippet_with_context};
use clippy_utils::ty::needs_ordered_drop;
use clippy_utils::visitors::{any_temporaries_need_ordered_drop, for_each_expr_without_closures};
use clippy_utils::{higher, peel_blocks};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::{
    Block, Destination, Expr, ExprKind, HirId, LetStmt, LoopSource, MatchSource, Pat, PatKind, Path, QPath, Stmt,
    StmtKind, Ty,
};
use rustc_lint::LateContext;
use std::fmt::Write;
use std::ops::ControlFlow;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, loop_block: &'tcx Block<'_>) {
    let (init, let_info, els) = match (loop_block.stmts, loop_block.expr) {
        ([stmt, ..], _) => match stmt.kind {
            StmtKind::Let(LetStmt {
                init: Some(e),
                els,
                pat,
                ty,
                ..
            }) => (*e, Some((*pat, *ty)), *els),
            StmtKind::Semi(e) | StmtKind::Expr(e) => (e, None, None),
            _ => return,
        },
        ([], Some(e)) => (e, None, None),
        _ => return,
    };
    let has_trailing_exprs = loop_block.stmts.len() + usize::from(loop_block.expr.is_some()) > 1;

    let (let_pat, let_expr, inner_expr, hoistable_stmts) = if let Some(if_let) = higher::IfLet::hir(cx, init)
        && let Some(else_expr) = if_let.if_else
        && is_simple_break_expr(else_expr)
    {
        (if_let.let_pat, if_let.let_expr, Some(if_let.if_then), None)
    } else if els.is_some_and(is_simple_break_block)
        && let Some((pat, _)) = let_info
    {
        (pat, init, None, None)
    } else if let Some(els_block) = els
        && let Some((pat, _)) = let_info
        && let Some(hoistable) = extract_hoistable_stmts(els_block, expr.hir_id)
    {
        (pat, init, None, Some(hoistable))
    } else if let ExprKind::Match(scrutinee, [arm1, arm2], MatchSource::Normal) = init.kind
        && arm1.guard.is_none()
        && arm2.guard.is_none()
        && is_simple_break_expr(arm2.body)
    {
        (arm1.pat, scrutinee, Some(arm1.body), None)
    } else {
        return;
    };

    if (has_trailing_exprs || hoistable_stmts.is_some())
        && (needs_ordered_drop(cx, cx.typeck_results().expr_ty(let_expr))
            || any_temporaries_need_ordered_drop(cx, let_expr))
    {
        return;
    }

    could_be_while_let(cx, expr, loop_block, let_info, let_pat, let_expr, inner_expr, hoistable_stmts);
}

/// Checks if `block` contains a single unlabeled `break` expression or statement, possibly embedded
/// inside other blocks.
fn is_simple_break_block(block: &Block<'_>) -> bool {
    match (block.stmts, block.expr) {
        ([s], None) => matches!(s.kind, StmtKind::Expr(e) | StmtKind::Semi(e) if is_simple_break_expr(e)),
        ([], Some(e)) => is_simple_break_expr(e),
        _ => false,
    }
}

/// Checks if `expr` contains a single unlabeled `break` expression or statement, possibly embedded
/// inside other blocks.
fn is_simple_break_expr(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Block(b, _) => is_simple_break_block(b),
        ExprKind::Break(dest, None) => dest.label.is_none(),
        _ => false,
    }
}

#[expect(clippy::too_many_arguments)]
fn could_be_while_let<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    loop_block: &'tcx Block<'_>,
    let_info: Option<(&'tcx Pat<'tcx>, Option<&'tcx Ty<'tcx>>)>,
    let_pat: &'tcx Pat<'tcx>,
    let_expr: &'tcx Expr<'tcx>,
    inner_expr: Option<&'tcx Expr<'tcx>>,
    hoistable_stmts: Option<&'tcx [Stmt<'tcx>]>,
) {
    let indent = snippet_indent(cx, expr.span).unwrap_or_default();

    let label_prefix = if let ExprKind::Loop(_, Some(label), LoopSource::Loop, _) = expr.kind {
        format!("{}: ", label.ident)
    } else {
        String::new()
    };

    // NOTE: we used to build a body here instead of using
    // ellipsis, this was removed because:
    // 1) it was ugly with big bodies;
    // 2) it was not indented properly;
    // 3) it wasn't very smart (see #675).
    let inner_content = if let Some(((pat, ty), inner_expr)) = let_info.zip(inner_expr)
        // Prevent trivial reassignments such as `let x = x;` or `let _ = …;`, but
        // keep them if the type has been explicitly specified.
        && (!is_trivial_assignment(pat, peel_blocks(inner_expr)) || ty.is_some())
        && let Some(pat_str) = snippet_opt(cx, pat.span)
        && let Some(init_str) = snippet_opt(cx, peel_blocks(inner_expr).span)
    {
        let ty_str = ty
            .map(|ty| format!(": {}", snippet(cx, ty.span, "_")))
            .unwrap_or_default();
        format!("\n{indent}    let {pat_str}{ty_str} = {init_str};\n{indent}    ..\n{indent}")
    } else {
        " .. ".into()
    };

    let hoisted_content = if let Some(stmts) = hoistable_stmts {
        let mut hoisted = String::new();
        let outer_ctxt = expr.span.ctxt();
        for stmt in stmts {
            let (stmt_str, _) = snippet_with_context(cx, stmt.span, outer_ctxt, "..", &mut Applicability::Unspecified);
            let semi = if matches!(stmt.kind, StmtKind::Semi(_)) {
                ";"
            } else {
                ""
            };
            let reindented = reindent_multiline(&stmt_str, true, Some(indent.len()));
            let _ = write!(hoisted, "\n{indent}{reindented}{semi}");
        }
        hoisted
    } else {
        String::new()
    };

    let human_suggestion = format!(
        "{label_prefix}while let {} = {} {{{inner_content}}}{hoisted_content}",
        snippet(cx, let_pat.span, ".."),
        snippet(cx, let_expr.span, ".."),
    );

    span_lint_and_then(
        cx,
        WHILE_LET_LOOP,
        expr.span,
        "this loop could be written as a `while let` loop",
        |diag| {
            diag.span_suggestion(expr.span, "try", &human_suggestion, Applicability::HasPlaceholders);

            if inner_expr.is_none() {
                let while_let_header = format!(
                    "{label_prefix}while let {} = {} {{",
                    snippet(cx, let_pat.span, ".."),
                    snippet(cx, let_expr.span, ".."),
                );

                let first_stmt_span = loop_block.stmts[0].span;
                let replace_span = expr.span.with_hi(first_stmt_span.hi());

                let mut parts = vec![(replace_span, while_let_header)];

                if !hoisted_content.is_empty() {
                    parts.push((expr.span.shrink_to_hi(), hoisted_content.clone()));
                }

                diag.tool_only_multipart_suggestion("try", parts, Applicability::MachineApplicable);
            }
        },
    );
}

fn is_trivial_assignment(pat: &Pat<'_>, init: &Expr<'_>) -> bool {
    match (pat.kind, init.kind) {
        (PatKind::Wild, _) => true,
        (
            PatKind::Binding(BindingMode::NONE, _, pat_ident, None),
            ExprKind::Path(QPath::Resolved(None, Path { segments: [init], .. })),
        ) => pat_ident.name == init.ident.name,
        _ => false,
    }
}

/// Checks if a block ends with an unlabeled `break` and returns the statements before it,
/// or `None` if any statement before the break contains a `break` or `continue` targeting
/// the loop identified by `loop_id`.
fn extract_hoistable_stmts<'tcx>(block: &'tcx Block<'tcx>, loop_id: HirId) -> Option<&'tcx [Stmt<'tcx>]> {
    let stmts_before_break = match (block.stmts, block.expr) {
        (stmts, Some(e)) if is_simple_break_expr(e) => stmts,
        (stmts, None) if !stmts.is_empty() => {
            let (last, rest) = stmts.split_last()?;
            match last.kind {
                StmtKind::Expr(e) | StmtKind::Semi(e) if is_simple_break_expr(e) => rest,
                _ => return None,
            }
        },
        _ => return None,
    };

    if stmts_before_break.is_empty() {
        return None;
    }

    // Reject statements containing a `break`/`continue` targeting the loop
    // being transformed. Breaks/continues to other loops and returns are fine to hoist.
    let has_problematic_control_flow = stmts_before_break.iter().any(|stmt| {
        for_each_expr_without_closures(stmt, |e| match e.kind {
            ExprKind::Break(Destination { target_id: Ok(id), .. }, _)
            | ExprKind::Continue(Destination { target_id: Ok(id), .. }) if id == loop_id => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        })
        .is_some()
    });

    (!has_problematic_control_flow).then_some(stmts_before_break)
}
