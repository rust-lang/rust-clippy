use super::WHILE_LET_LOOP;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet, snippet_indent, snippet_opt, snippet_with_context};
use clippy_utils::ty::needs_ordered_drop;
use clippy_utils::visitors::{any_temporaries_need_ordered_drop, for_each_expr_without_closures};
use clippy_utils::{higher, peel_blocks};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::{
    Block, Destination, Expr, ExprKind, LetStmt, MatchSource, Pat, PatKind, Path, QPath, Stmt, StmtKind, Ty,
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

    if let Some(if_let) = higher::IfLet::hir(cx, init)
        && let Some(else_expr) = if_let.if_else
        && is_simple_break_expr(else_expr)
    {
        could_be_while_let(
            cx,
            expr,
            WhileLetInfo {
                let_pat: if_let.let_pat,
                let_expr: if_let.let_expr,
                has_trailing_exprs,
                let_info,
                inner_expr: Some(if_let.if_then),
                hoistable_stmts: None,
            },
        );
    } else if els.is_some_and(is_simple_break_block)
        && let Some((pat, _)) = let_info
    {
        could_be_while_let(
            cx,
            expr,
            WhileLetInfo {
                let_pat: pat,
                let_expr: init,
                has_trailing_exprs,
                let_info,
                inner_expr: None,
                hoistable_stmts: None,
            },
        );
    } else if let Some(els_block) = els
        && let Some((pat, _)) = let_info
        && let Some(hoistable) = extract_hoistable_stmts(els_block)
    {
        could_be_while_let(
            cx,
            expr,
            WhileLetInfo {
                let_pat: pat,
                let_expr: init,
                has_trailing_exprs,
                let_info,
                inner_expr: None,
                hoistable_stmts: Some(hoistable),
            },
        );
    } else if let ExprKind::Match(scrutinee, [arm1, arm2], MatchSource::Normal) = init.kind
        && arm1.guard.is_none()
        && arm2.guard.is_none()
        && is_simple_break_expr(arm2.body)
    {
        could_be_while_let(
            cx,
            expr,
            WhileLetInfo {
                let_pat: arm1.pat,
                let_expr: scrutinee,
                has_trailing_exprs,
                let_info,
                inner_expr: Some(arm1.body),
                hoistable_stmts: None,
            },
        );
    }
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

#[derive(Copy, Clone)]
struct WhileLetInfo<'tcx> {
    let_pat: &'tcx Pat<'tcx>,
    let_expr: &'tcx Expr<'tcx>,
    has_trailing_exprs: bool,
    let_info: Option<(&'tcx Pat<'tcx>, Option<&'tcx Ty<'tcx>>)>,
    inner_expr: Option<&'tcx Expr<'tcx>>,
    hoistable_stmts: Option<&'tcx [Stmt<'tcx>]>,
}

fn could_be_while_let<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, info: WhileLetInfo<'tcx>) {
    let WhileLetInfo {
        let_pat,
        let_expr,
        has_trailing_exprs,
        let_info,
        inner_expr,
        hoistable_stmts,
    } = info;

    if has_trailing_exprs
        && (needs_ordered_drop(cx, cx.typeck_results().expr_ty(let_expr))
            || any_temporaries_need_ordered_drop(cx, let_expr))
    {
        // Switching to a `while let` loop will extend the lifetime of some values.
        return;
    }

    let indent = snippet_indent(cx, expr.span).unwrap_or_default();

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

    // Build the hoisted statements string in case we have statements to hoist
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
            let _ = write!(hoisted, "\n{indent}{stmt_str}{semi}");
        }
        hoisted
    } else {
        String::new()
    };

    span_lint_and_sugg(
        cx,
        WHILE_LET_LOOP,
        expr.span,
        "this loop could be written as a `while let` loop",
        "try",
        format!(
            "while let {} = {} {{{inner_content}}}{hoisted_content}",
            snippet(cx, let_pat.span, ".."),
            snippet(cx, let_expr.span, ".."),
        ),
        Applicability::HasPlaceholders,
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

/// Checks if a block ends with an unlabeled `break` and returns the statements before it..
/// or `None` if the block doesn't end with a simple break or if any statement before
/// the break could exit the loop (via return, labeled break, etc.).
fn extract_hoistable_stmts<'tcx>(block: &'tcx Block<'tcx>) -> Option<&'tcx [Stmt<'tcx>]> {
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

    // Check that none of the statements before break contain return, labeled break,
    // or other control flow that could exit the loop differently
    let has_early_exit = stmts_before_break.iter().any(|stmt| {
        for_each_expr_without_closures(stmt, |e| match e.kind {
            ExprKind::Ret(..)
            | ExprKind::Break(Destination { label: Some(_), .. }, _)
            | ExprKind::Continue(Destination { label: Some(_), .. }) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        })
        .is_some()
    });

    (!has_early_exit).then_some(stmts_before_break)
}
