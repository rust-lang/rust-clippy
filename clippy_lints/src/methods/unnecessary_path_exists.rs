use super::UNNECESSARY_PATH_EXISTS;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef;
use clippy_utils::{SpanlessEq, get_enclosing_block, get_parent_expr, higher, path_to_local_with_projections, sym};
use rustc_hir::{BinOpKind, Expr, ExprKind, MatchSource, Node, PatKind, StmtKind};
use rustc_lint::LateContext;
use rustc_span::{Span, SyntaxContext};

/// `expr` is a `.exists()` call on `recv`. Find out whether it's used either
/// directly (or through a chain of `&&`) as an `if` condition, or stored in a
/// `let` binding that's immediately checked by the following `if`, and if so
/// look for a redundant filesystem operation in the `then` branch.
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, recv: &'tcx Expr<'tcx>) {
    if is_path_method_call(cx, expr)
        && let Some((then, ctxt)) = if_then_from_condition(cx, expr).or_else(|| if_then_from_stored_bool(cx, expr))
        && let Some(fs_call_span) = find_fs_call(cx, then, recv, ctxt)
    {
        span_lint_and_then(
            cx,
            UNNECESSARY_PATH_EXISTS,
            expr.span,
            "unnecessary `Path::exists` before a filesystem operation on the same path",
            |diag| {
                diag.span_note(fs_call_span, "the filesystem operation is here");
                diag.help(
                    "the `exists()` check is redundant and creates a TOCTOU race condition; \
                     consider removing it and handling the error from the filesystem operation directly",
                );
            },
        );
    }
}

/// If `current` is the operand of a `?` operator (i.e. `current?`), returns the
/// `Match` expression that the desugaring produces, so callers can keep
/// climbing from there. `EXPR?` lowers to
/// `Match(Call(<lang item Try::branch>, [EXPR]), _, TryDesugar(call_hir_id))`,
/// so this is recognized structurally via `MatchSource::TryDesugar`, not by
/// name/string matching on the call.
fn peel_try_desugar<'tcx>(cx: &LateContext<'tcx>, current: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let call_expr = get_parent_expr(cx, current)?;
    let ExprKind::Call(_, [arg]) = call_expr.kind else {
        return None;
    };
    if arg.hir_id != current.hir_id {
        return None;
    }
    let match_expr = get_parent_expr(cx, call_expr)?;
    if let ExprKind::Match(_, _, MatchSource::TryDesugar(scrutinee_id)) = match_expr.kind
        && scrutinee_id == call_expr.hir_id
    {
        Some(match_expr)
    } else {
        None
    }
}

/// Repeatedly applies [`peel_try_desugar`], returning the outermost expression
/// once no more `?` layers can be peeled.
fn peel_try_desugars<'tcx>(cx: &LateContext<'tcx>, mut current: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    while let Some(match_expr) = peel_try_desugar(cx, current) {
        current = match_expr;
    }
    current
}

/// Climbs through any enclosing `&&` chain (peeling a leading `?`, e.g. from
/// `path.try_exists()?`, first) looking for an enclosing `if` whose condition
/// is exactly the expression we climbed to.
fn if_then_from_condition<'tcx>(
    cx: &LateContext<'tcx>,
    exists_expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, SyntaxContext)> {
    let mut current = peel_try_desugars(cx, exists_expr);
    loop {
        let parent = get_parent_expr(cx, current)?;
        match parent.kind {
            ExprKind::Binary(op, lhs, rhs)
                if op.node == BinOpKind::And && (lhs.hir_id == current.hir_id || rhs.hir_id == current.hir_id) =>
            {
                current = parent;
            },
            _ => {
                let higher::If { cond, then, .. } = higher::If::hir(parent)?;
                return (cond.hir_id == current.hir_id && !parent.span.from_expansion())
                    .then(|| (then, parent.span.ctxt()));
            },
        }
    }
}

/// Handles `let b = path.exists(); if b { ... }` (or the `try_exists()?`
/// equivalent), where the `if` immediately follows the `let` in the same
/// block.
fn if_then_from_stored_bool<'tcx>(
    cx: &LateContext<'tcx>,
    exists_expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, SyntaxContext)> {
    let outer = peel_try_desugars(cx, exists_expr);
    let Node::LetStmt(local) = cx.tcx.parent_hir_node(outer.hir_id) else {
        return None;
    };
    let PatKind::Binding(_, binding_id, _, _) = local.pat.kind else {
        return None;
    };

    let block = get_enclosing_block(cx, local.hir_id)?;
    if block.span.from_expansion() {
        return None;
    }
    let idx = block
        .stmts
        .iter()
        .position(|stmt| matches!(stmt.kind, StmtKind::Let(l) if l.hir_id == local.hir_id))?;
    let next_expr = match block.stmts.get(idx + 1) {
        Some(stmt) => match stmt.kind {
            StmtKind::Expr(e) | StmtKind::Semi(e) => Some(e),
            StmtKind::Let(_) | StmtKind::Item(_) => None,
        },
        None => block.expr,
    }?;

    let higher::If { cond, then, .. } = higher::If::hir(next_expr)?;
    (path_to_local_with_projections(cond) == Some(binding_id)).then(|| (then, next_expr.span.ctxt()))
}

fn is_fs_method_name(name: rustc_span::Symbol) -> bool {
    matches!(
        name,
        sym::canonicalize
            | sym::is_dir
            | sym::is_file
            | sym::is_symlink
            | sym::metadata
            | sym::read_dir
            | sym::read_link
            | sym::symlink_metadata
    )
}

/// Returns `true` if `expr` is a method call that resolves to a method defined
/// on `std::path::Path` (handles any type that derefs to `Path`, e.g. `PathBuf`).
fn is_path_method_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|def_id| {
            let parent = cx.tcx.parent(def_id);
            cx.tcx
                .type_of(parent)
                .instantiate_identity()
                .skip_norm_wip()
                .is_diag_item(cx, sym::Path)
        })
}

/// Searches the `then` block of the `if` for the first filesystem method call
/// on the same receiver as the `exists()` check.
fn find_fs_call<'tcx>(
    cx: &LateContext<'tcx>,
    then: &'tcx Expr<'tcx>,
    path_recv: &'tcx Expr<'tcx>,
    ctxt: SyntaxContext,
) -> Option<Span> {
    let ExprKind::Block(block, _) = then.kind else {
        return None;
    };
    for stmt in block.stmts {
        let candidate = match stmt.kind {
            StmtKind::Expr(e) | StmtKind::Semi(e) => Some(e),
            StmtKind::Let(local) => local.init,
            StmtKind::Item(_) => None,
        };
        if let Some(span) = candidate.and_then(|e| find_fs_call_in_expr(cx, e, path_recv, ctxt)) {
            return Some(span);
        }
    }
    block.expr.and_then(|e| find_fs_call_in_expr(cx, e, path_recv, ctxt))
}

/// Peels through method chains (e.g. `.metadata().unwrap()`) and the `?` operator
/// desugaring to find a filesystem method call on `path_recv`.
fn find_fs_call_in_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    path_recv: &'tcx Expr<'tcx>,
    ctxt: SyntaxContext,
) -> Option<Span> {
    match expr.kind {
        ExprKind::MethodCall(method_seg, recv, _, _) => {
            if is_fs_method_name(method_seg.ident.name)
                && is_path_method_call(cx, expr)
                && SpanlessEq::new(cx).eq_expr(ctxt, recv, path_recv)
            {
                return Some(expr.span);
            }
            // Peel through chains like `.metadata().unwrap()` or `.metadata().ok()`
            find_fs_call_in_expr(cx, recv, path_recv, ctxt)
        },
        // The `?` operator desugars to:
        //   Match(Call(TryTraitBranch, [inner_expr]), ..., TryDesugar)
        // so we extract `inner_expr` and keep searching.
        ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) => {
            if let ExprKind::Call(_, [inner_expr]) = scrutinee.kind {
                find_fs_call_in_expr(cx, inner_expr, path_recv, ctxt)
            } else {
                None
            }
        },
        _ => None,
    }
}
