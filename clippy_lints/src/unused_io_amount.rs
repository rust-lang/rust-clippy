use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_res_lang_ctor, is_trait_method, match_trait_method, paths};
use hir::{ExprKind, PatKind};
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unused written/read amount.
    ///
    /// ### Why is this bad?
    /// `io::Write::write(_vectored)` and
    /// `io::Read::read(_vectored)` are not guaranteed to
    /// process the entire buffer. They return how many bytes were processed, which
    /// might be smaller
    /// than a given buffer's length. If you don't need to deal with
    /// partial-write/read, use
    /// `write_all`/`read_exact` instead.
    ///
    /// When working with asynchronous code (either with the `futures`
    /// crate or with `tokio`), a similar issue exists for
    /// `AsyncWriteExt::write()` and `AsyncReadExt::read()` : these
    /// functions are also not guaranteed to process the entire
    /// buffer.  Your code should either handle partial-writes/reads, or
    /// call the `write_all`/`read_exact` methods on those traits instead.
    ///
    /// ### Known problems
    /// Detects only common patterns.
    ///
    /// ### Examples
    /// ```rust,ignore
    /// use std::io;
    /// fn foo<W: io::Write>(w: &mut W) -> io::Result<()> {
    ///     // must be `w.write_all(b"foo")?;`
    ///     w.write(b"foo")?;
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub UNUSED_IO_AMOUNT,
    correctness,
    "unused written/read amount"
}

declare_lint_pass!(UnusedIoAmount => [UNUSED_IO_AMOUNT]);

#[derive(Copy, Clone)]
enum IoOp {
    AsyncWrite(bool),
    AsyncRead(bool),
    SyncRead(bool),
    SyncWrite(bool),
}

/// We need to check that read/write call sizes are handled.
/// We need to be careful with this to avoid false positives.
/// If we do the check directly on Expr, we will miss cases like:
/// `
/// result = reader.read(&mut buf).unwrap();`
/// because we are oblivious to the fact that the read amount gets bound to `result`.
///
/// Here's what we will do:
///     1. Get the callsite Expr
///     2. Check if it's a call to read/write
///     3. Decide whether it should be linted based on the surrounding context, e.g. arms in a match
///        statement, etc
///     4. Lint if necessary
/// This is different to the current impl. Current impl which is limited in the match statements it
/// catches and doesn't catch cases where match is just returned.
///
/// We will use logic similar to
impl<'tcx> LateLintPass<'tcx> for UnusedIoAmount {
    /// We perform the filtering on the block level.
    /// If we want to catch match and if expressions that act as returns of the block
    ///   we need to check them at `check_expr` or `check_block` as they are not stmts
    ///   but we can't check them at `check_expr` because we need the broader context
    ///   because we should do this only for the final expression of the block, and not for
    ///   `StmtKind::Local` which binds values => the io amount is used.
    ///
    /// To check for unused io amount in stmts, we only consider `StmtKind::Semi`.
    /// `StmtKind::Local` is not considered because it binds values => the io amount is used.
    /// `StmtKind::Expr` is not considered because requires unit type => the io amount is used.
    /// `StmtKind::Item` is not considered because it's not an expression.
    ///
    /// We then check the individual expressions via `check_expr`. We use the same logic for
    /// semi expressions and the final expression as we need to check match and if expressions
    /// for binding of the io amount to `Ok(_)`.
    ///
    /// We explicitly check for the match source to be Normal as it needs special logic
    /// to consider the arms, and we want to avoid breaking the logic for situations where things
    /// get desugared to match.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        for stmt in block.stmts {
            if let hir::StmtKind::Semi(exp) = stmt.kind {
                check_expr(cx, exp);
            }
        }

        if let Some(exp) = block.expr {
            match exp.kind {
                hir::ExprKind::If(_, _, _) | hir::ExprKind::Match(_, _, _) => check_expr(cx, exp),
                _ => {},
            }
        }
    }
}

fn check_expr<'a>(cx: &LateContext<'a>, expr: &'a hir::Expr<'a>) {
    match expr.kind {
        hir::ExprKind::If(cond, _, _)
            if let ExprKind::Let(hir::Let { pat, init, .. }) = cond.kind
                && pattern_is_okay_underscore(cx, pat)
                && let Some(op) = should_lint(cx, init) =>
        {
            emit_lint(cx, cond.span, op, &[pat.span]);
        },
        hir::ExprKind::Match(expr, arms, hir::MatchSource::Normal) if let Some(op) = should_lint(cx, expr) => {
            let found_arms: Vec<_> = arms
                .iter()
                .filter_map(|arm| {
                    if pattern_is_okay_underscore(cx, arm.pat) {
                        Some(arm.span)
                    } else {
                        None
                    }
                })
                .collect();
            if !found_arms.is_empty() {
                emit_lint(cx, expr.span, op, found_arms.as_slice());
            }
        },
        _ if let Some(op) = should_lint(cx, expr) => {
            emit_lint(cx, expr.span, op, &[]);
        },
        _ => {},
    };
}

fn should_lint<'a>(cx: &LateContext<'a>, mut inner: &'a hir::Expr<'a>) -> Option<IoOp> {
    inner = unpack_match(inner);
    inner = unpack_try(inner);
    inner = unpack_call_chain(inner);
    inner = unpack_await(inner);
    // we type-check it to get whether it's a read/write or their vectorized forms
    // and keep only the ones that are produce io amount
    check_io_mode(cx, inner)
}

fn is_okay_pat(cx: &LateContext<'_>, pat: &hir::Pat<'_>) -> bool {
    if let PatKind::TupleStruct(ref path, [inner_pat], ddp) = pat.kind
        && ddp.as_opt_usize().is_none()
        && is_res_lang_ctor(cx, cx.qpath_res(path, inner_pat.hir_id), hir::LangItem::ResultOk)
    {
        return true;
    }
    false
}

fn pattern_is_okay_underscore(cx: &LateContext<'_>, pat: &hir::Pat<'_>) -> bool {
    is_okay_pat(cx, pat)
        && matches!(
            pat.kind,
            PatKind::TupleStruct(
                _,
                [hir::Pat {
                    kind: PatKind::Wild,
                    ..
                }],
                _
            )
        )
}

fn unpack_call_chain<'a>(mut expr: &'a hir::Expr<'a>) -> &'a hir::Expr<'a> {
    while let hir::ExprKind::MethodCall(path, receiver, ..) = expr.kind {
        if matches!(
            path.ident.as_str(),
            "or" | "or_else" | "ok" | "expect" | "unwrap" | "unwrap_or" | "unwrap_or_else" | "is_ok" | "is_err"
        ) {
            expr = receiver;
        } else {
            break;
        }
    }
    expr
}

fn unpack_try<'a>(mut expr: &'a hir::Expr<'a>) -> &'a hir::Expr<'a> {
    while let hir::ExprKind::Call(func, [ref arg_0, ..]) = expr.kind
        && matches!(
            func.kind,
            hir::ExprKind::Path(hir::QPath::LangItem(hir::LangItem::TryTraitBranch, ..))
        )
    {
        expr = arg_0;
    }
    expr
}

fn unpack_match<'a>(mut expr: &'a hir::Expr<'a>) -> &'a hir::Expr<'a> {
    while let hir::ExprKind::Match(res, _, _) = expr.kind {
        expr = res;
    }
    expr
}

/// If `expr` is an (e).await, return the inner expression "e" that's being
/// waited on.  Otherwise return None.
fn unpack_await<'a>(expr: &'a hir::Expr<'a>) -> &hir::Expr<'a> {
    if let hir::ExprKind::Match(expr, _, hir::MatchSource::AwaitDesugar) = expr.kind {
        if let hir::ExprKind::Call(func, [ref arg_0, ..]) = expr.kind {
            if matches!(
                func.kind,
                hir::ExprKind::Path(hir::QPath::LangItem(hir::LangItem::IntoFutureIntoFuture, ..))
            ) {
                return arg_0;
            }
        }
    }
    expr
}

///
/// Check whether the current expr is a function call for an IO operation
fn check_io_mode(cx: &LateContext<'_>, call: &hir::Expr<'_>) -> Option<IoOp> {
    let hir::ExprKind::MethodCall(path, ..) = call.kind else {
        return None;
    };

    let vectorized = match path.ident.as_str() {
        "write_vectored" | "read_vectored" => true,
        "write" | "read" => false,
        _ => {
            return None;
        },
    };

    match (
        match_trait_method(cx, call, &paths::FUTURES_IO_ASYNCREADEXT)
            || match_trait_method(cx, call, &paths::TOKIO_IO_ASYNCREADEXT),
        match_trait_method(cx, call, &paths::TOKIO_IO_ASYNCWRITEEXT)
            || match_trait_method(cx, call, &paths::FUTURES_IO_ASYNCWRITEEXT),
        is_trait_method(cx, call, sym::IoRead),
        is_trait_method(cx, call, sym::IoWrite),
    ) {
        (true, _, _, _) => Some(IoOp::AsyncRead(vectorized)),
        (_, true, _, _) => Some(IoOp::AsyncWrite(vectorized)),
        (_, _, true, _) => Some(IoOp::SyncRead(vectorized)),
        (_, _, _, true) => Some(IoOp::SyncWrite(vectorized)),
        _ => None,
    }
}

fn emit_lint(cx: &LateContext<'_>, span: Span, op: IoOp, wild_cards: &[Span]) {
    let (msg, help) = match op {
        IoOp::AsyncRead(false) => (
            "read amount is not handled",
            Some("use `AsyncReadExt::read_exact` instead, or handle partial reads"),
        ),
        IoOp::SyncRead(false) => (
            "read amount is not handled",
            Some("use `Read::read_exact` instead, or handle partial reads"),
        ),
        IoOp::SyncWrite(false) => (
            "written amount is not handled",
            Some("use `Write::write_all` instead, or handle partial writes"),
        ),
        IoOp::AsyncWrite(false) => (
            "written amount is not handled",
            Some("use `AsyncWriteExt::write_all` instead, or handle partial writes"),
        ),
        IoOp::SyncRead(true) | IoOp::AsyncRead(true) => ("read amount is not handled", None),
        IoOp::SyncWrite(true) | IoOp::AsyncWrite(true) => ("written amount is not handled", None),
    };

    span_lint_and_then(cx, UNUSED_IO_AMOUNT, span, msg, |diag| {
        if let Some(help_str) = help {
            diag.help(help_str);
        }
        for span in wild_cards {
            diag.span_note(*span, "io bytes are ignored");
        }
    });
}
