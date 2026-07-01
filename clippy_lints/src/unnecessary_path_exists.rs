use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef;
use clippy_utils::{SpanlessEq, higher, path_to_local_with_projections, sym};
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, MatchSource, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::{Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `Path::exists` immediately before a filesystem
    /// operation on the same path.
    ///
    /// ### Why is this bad?
    /// Calling `exists()` and then performing a filesystem operation on the same
    /// path is a classic Time-Of-Check to Time-Of-Use (TOCTOU) race condition.
    /// Between the two calls another process can add, remove, or replace the
    /// file, making the result of `exists()` stale. The filesystem operation
    /// itself will indicate whether the path exists via its return value, making
    /// the prior `exists()` check both redundant and dangerous.
    ///
    /// ### Example
    /// ```rust,no_run
    /// # use std::path::Path;
    /// # fn example(path: &Path) {
    /// if path.exists() {
    ///     let metadata = path.metadata().unwrap();
    ///     // use metadata ...
    /// }
    /// # }
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// # use std::path::Path;
    /// # fn example(path: &Path) {
    /// if let Ok(metadata) = path.metadata() {
    ///     // use metadata ...
    /// }
    /// # }
    /// ```
    ///
    /// ### Known problems
    /// - Does not detect `std::fs` free functions used inside the block
    ///   (e.g. `fs::read(path)`, `fs::File::open(path)`), only method calls on
    ///   the path receiver itself.
    /// - Does not detect `Path::try_exists()` (stabilized in Rust 1.63): the `?`
    ///   operator in the condition desugars to a `Match` node, so the condition
    ///   is not seen as a simple `.exists()` call.
    /// - For the stored-bool variant (`let b = path.exists(); /* other stmts */;
    ///   if b { ... }`), only detects when the `if` immediately follows the `let`.
    #[clippy::version = "1.98.0"]
    pub UNNECESSARY_PATH_EXISTS,
    nursery,
    "calling `Path::exists` before a filesystem operation creates a TOCTOU race"
}

declare_lint_pass!(UnnecessaryPathExists => [UNNECESSARY_PATH_EXISTS]);

/// `Path`/`PathBuf` methods that each initiate a fresh syscall.
const FS_METHODS: &[&str] = &[
    "canonicalize",
    "is_dir",
    "is_file",
    "is_symlink",
    "metadata",
    "read_dir",
    "read_link",
    "symlink_metadata",
];

impl<'tcx> LateLintPass<'tcx> for UnnecessaryPathExists {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }

        if let Some(higher::If { cond, then, .. }) = higher::If::hir(expr)
            && let Some((path_recv, exists_span)) = extract_exists_receiver(cx, cond)
            && let Some(fs_call_span) = find_fs_call(cx, then, path_recv, expr.span.ctxt())
        {
            emit_lint(cx, exists_span, fs_call_span);
        }
    }

    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if block.span.from_expansion() {
            return;
        }

        let stmts = block.stmts;
        for (idx, stmt) in stmts.iter().enumerate() {
            // Match `let b = path.exists()`
            let StmtKind::Let(local) = stmt.kind else {
                continue;
            };
            let Some(init) = local.init else { continue };
            let Some((path_recv, exists_span)) = extract_exists_receiver(cx, init) else {
                continue;
            };
            // Simple binding pattern: `let b = ...` (not destructuring)
            let PatKind::Binding(_, binding_id, _, _) = local.pat.kind else {
                continue;
            };

            // Find the immediately following expression
            let next_expr = if idx + 1 < stmts.len() {
                match stmts[idx + 1].kind {
                    StmtKind::Expr(e) | StmtKind::Semi(e) => e,
                    _ => continue,
                }
            } else if let Some(e) = block.expr {
                e
            } else {
                continue;
            };

            if let Some(higher::If { cond, then, .. }) = higher::If::hir(next_expr)
                && path_to_local_with_projections(cond) == Some(binding_id)
                && let Some(fs_call_span) = find_fs_call(cx, then, path_recv, next_expr.span.ctxt())
            {
                emit_lint(cx, exists_span, fs_call_span);
            }
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, exists_span: Span, fs_call_span: Span) {
    span_lint_and_then(
        cx,
        UNNECESSARY_PATH_EXISTS,
        exists_span,
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

/// Walks a condition expression to find a `.exists()` call on a `Path`/`PathBuf`
/// receiver. Returns the receiver expression and the span of the `.exists()` call.
/// Recurses through `&&` chains so compound conditions are handled.
fn extract_exists_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, Span)> {
    match expr.kind {
        ExprKind::MethodCall(seg, recv, [], _)
            if seg.ident.name == sym::exists && !expr.span.from_expansion() =>
        {
            let ty = cx.typeck_results().expr_ty(recv).peel_refs();
            if matches!(ty.opt_diag_name(cx), Some(sym::Path | sym::PathBuf)) {
                Some((recv, expr.span))
            } else {
                None
            }
        },
        ExprKind::Binary(op, lhs, rhs) if op.node == BinOpKind::And => {
            extract_exists_receiver(cx, lhs).or_else(|| extract_exists_receiver(cx, rhs))
        },
        _ => None,
    }
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
        if let Some(span) = find_fs_call_in_stmt(cx, stmt, path_recv, ctxt) {
            return Some(span);
        }
    }
    block.expr.and_then(|e| find_fs_call_in_expr(cx, e, path_recv, ctxt))
}

fn find_fs_call_in_stmt<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
    path_recv: &'tcx Expr<'tcx>,
    ctxt: SyntaxContext,
) -> Option<Span> {
    match stmt.kind {
        StmtKind::Expr(e) | StmtKind::Semi(e) => find_fs_call_in_expr(cx, e, path_recv, ctxt),
        StmtKind::Let(local) => local
            .init
            .and_then(|init| find_fs_call_in_expr(cx, init, path_recv, ctxt)),
        StmtKind::Item(_) => None,
    }
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
            if FS_METHODS.contains(&method_seg.ident.name.as_str()) {
                let recv_ty = cx.typeck_results().expr_ty(recv).peel_refs();
                if matches!(recv_ty.opt_diag_name(cx), Some(sym::Path | sym::PathBuf))
                    && SpanlessEq::new(cx).eq_expr(ctxt, recv, path_recv)
                {
                    return Some(expr.span);
                }
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
