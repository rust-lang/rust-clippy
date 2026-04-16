use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{snippet, snippet_indent};
use clippy_utils::visitors::is_local_used;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{BlockCheckMode, ExprKind, HirId, LetStmt, Node, PatKind, QPath, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `let (a, b) = { let x = expr; (x, expr2) };` where the block
    /// can be collapsed into individual `let` statements.
    ///
    /// ### Why is this bad?
    /// The block adds unnecessary nesting and visual indirection. Directly binding
    /// each variable is cleaner and more idiomatic.
    ///
    /// ### Example
    /// ```no_run
    /// let (a, b) = {
    ///     let x = some_call();
    ///     (x, other_call())
    /// };
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = some_call();
    /// let b = other_call();
    /// ```
    #[clippy::version = "1.89.0"]
    pub COLLAPSIBLE_TUPLE_LET,
    style,
    "unnecessary block in `let` tuple destructuring"
}

declare_lint_pass!(CollapsibleTupleLet => [COLLAPSIBLE_TUPLE_LET]);

impl<'tcx> LateLintPass<'tcx> for CollapsibleTupleLet {
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'tcx>) {
        check(cx, local);
    }
}

fn check<'tcx>(cx: &LateContext<'tcx>, local: &'tcx LetStmt<'tcx>) {
    // 1. Outer pattern must be a tuple with no rest pattern (`..`)
    let PatKind::Tuple(outer_pats, rest) = local.pat.kind else {
        return;
    };
    if rest.as_opt_usize().is_some() {
        return;
    }

    // 2. No outer type annotation and no `let-else`
    if local.ty.is_some() || local.els.is_some() {
        return;
    }

    // 3. Each outer tuple element must be a simple irrefutable pattern
    //    (plain binding or wildcard) to avoid generating refutable patterns
    if !outer_pats
        .iter()
        .all(|p| matches!(p.kind, PatKind::Binding(_, _, _, None) | PatKind::Wild))
    {
        return;
    }

    // 4. Init must be a non-labeled, safe block expression
    let Some(init_expr) = local.init else {
        return;
    };
    let ExprKind::Block(block, None) = init_expr.kind else {
        return;
    };
    if block.rules != BlockCheckMode::DefaultBlock {
        return;
    }

    // 5. Block must have a trailing tuple expression (no naked semicolons at end)
    let Some(trailing_expr) = block.expr else {
        return;
    };
    let ExprKind::Tup(tup_elems) = trailing_expr.kind else {
        return;
    };

    // 6. Arity: outer pattern and trailing tuple must have the same element count
    if outer_pats.len() != tup_elems.len() || outer_pats.is_empty() {
        return;
    }

    // 7. All block statements must be simple `let x = init;` bindings:
    //    - single non-destructured binding (PatKind::Binding with no sub-pattern)
    //    - no type annotation
    //    - no `let-else`
    //    - must have an initializer
    //    - must not come from a macro expansion
    let mut block_locals: Vec<(HirId, _)> = Vec::with_capacity(block.stmts.len());
    for stmt in block.stmts {
        if stmt.span.from_expansion() {
            return;
        }
        if let StmtKind::Let(inner) = stmt.kind
            && inner.ty.is_none()
            && inner.els.is_none()
            && let PatKind::Binding(_, hir_id, _, None) = inner.pat.kind
            && let Some(inner_init) = inner.init
        {
            block_locals.push((hir_id, inner_init));
        } else {
            return;
        }
    }

    // 8. Collect block-local HirIds for fast lookup
    let block_local_ids: Vec<HirId> = block_locals.iter().map(|(id, _)| *id).collect();

    // 9. Walk the tuple elements to verify:
    //    a) Each block-local referenced in the tuple appears exactly once
    //    b) Block-local references appear in declaration order (to preserve evaluation order)
    //    c) Inline expressions (non-block-local) do not reference any block-local
    //    d) All block-local references precede any inline expressions in the tuple
    //       (so that moving block-local inits to their tuple positions doesn't reorder
    //       side effects relative to inline expressions that come after them)
    let mut block_local_used = vec![false; block_locals.len()];
    let mut next_expected_idx: usize = 0;
    let mut seen_inline = false;

    for elem in tup_elems.iter() {
        // Try to resolve the element as a direct path to a block-local
        let resolved_idx = if let ExprKind::Path(QPath::Resolved(None, path)) = elem.kind
            && let Res::Local(hir_id) = path.res
        {
            block_local_ids.iter().position(|id| *id == hir_id)
        } else {
            None
        };

        if let Some(idx) = resolved_idx {
            // This element is a reference to a block-local
            if seen_inline {
                // A block-local appears after an inline expression; reordering inits
                // would change the evaluation order of side effects.
                return;
            }
            if idx != next_expected_idx {
                // Out of declaration order — reordering would change evaluation order.
                return;
            }
            if block_local_used[idx] {
                // Same block-local used twice in the tuple.
                return;
            }
            block_local_used[idx] = true;
            next_expected_idx += 1;
        } else {
            // Inline expression: must not reference any block-local
            if block_local_ids.iter().any(|id| is_local_used(cx, elem, *id)) {
                return;
            }
            seen_inline = true;
        }
    }

    // 10. Every block-local must appear exactly once in the tuple
    if block_local_used.iter().any(|&used| !used) {
        return;
    }

    // 11. No block-local may be referenced in any other block statement's initializer.
    //     (This ensures each block-local is used only in the trailing tuple.)
    for (id, _) in &block_locals {
        for stmt in block.stmts {
            if let StmtKind::Let(inner) = stmt.kind
                && let Some(inner_init) = inner.init
                && is_local_used(cx, inner_init, *id)
            {
                return;
            }
        }
    }

    // 12. Skip if the initializer or outer pattern span comes from a macro expansion
    if init_expr.span.from_expansion() || local.pat.span.from_expansion() {
        return;
    }

    // All checks passed. Get the parent `Stmt` span so the suggestion can
    // replace the entire statement including its trailing semicolon.
    let Some((_, Node::Stmt(parent_stmt))) = cx.tcx.hir_parent_iter(local.hir_id).next() else {
        return;
    };

    // Determine the indentation of the original statement so that continuation
    // lines in the multi-line suggestion are aligned with the first line.
    let indent = snippet_indent(cx, parent_stmt.span).unwrap_or_default();

    // Build the multi-line replacement: one `let pi = rhs;` per outer binding
    let mut parts: Vec<String> = Vec::with_capacity(outer_pats.len());
    for (pat, elem) in outer_pats.iter().zip(tup_elems.iter()) {
        let pat_snip = snippet(cx, pat.span, "_");

        // If the tuple element is a path to a block-local, use that local's
        // initializer as the RHS; otherwise use the element expression directly.
        let rhs_snip = if let ExprKind::Path(QPath::Resolved(None, path)) = elem.kind
            && let Res::Local(hir_id) = path.res
            && let Some((_, inner_init)) = block_locals.iter().find(|(id, _)| *id == hir_id)
        {
            snippet(cx, inner_init.span, "_")
        } else {
            snippet(cx, elem.span, "_")
        };

        parts.push(format!("let {pat_snip} = {rhs_snip};"));
    }

    // Join with newlines, prepending indentation to all lines after the first
    // (the first line's indentation is provided by the surrounding source).
    let replacement = parts.join(&format!("\n{indent}"));

    span_lint_and_then(
        cx,
        COLLAPSIBLE_TUPLE_LET,
        local.span,
        "this block can be collapsed into individual `let` statements",
        |diag| {
            diag.span_suggestion(
                parent_stmt.span,
                "collapse into separate `let` bindings",
                replacement,
                Applicability::MachineApplicable,
            );
        },
    );
}
