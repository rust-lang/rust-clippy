use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::visitors::for_each_local_use_after_expr;
use clippy_utils::{fn_def_id, match_any_def_paths, match_def_path, paths};
use rustc_ast::Mutability;
use rustc_hir::{Expr, ExprKind, Node, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Looks for code that spawns a process but never calls `wait()` on the child.
    ///
    /// ### Why is this bad?
    /// As explained in the [standard library documentation](https://doc.rust-lang.org/stable/std/process/struct.Child.html#warning),
    /// calling `wait()` is necessary on Unix platforms to properly release all OS resources associated with the process.
    /// Not doing so will effectively leak process IDs and/or other limited global resources,
    /// which can eventually lead to resource exhaustion, so it's recommended to call `wait()` in long-running applications.
    /// Such processes are called "zombie processes".
    ///
    /// ### Example
    /// ```rust
    /// use std::process::Command;
    ///
    /// let _child = Command::new("ls").spawn().expect("failed to execute child");
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::process::Command;
    ///
    /// let mut child = Command::new("ls").spawn().expect("failed to execute child");
    /// child.wait().expect("failed to wait on child");
    /// ```
    #[clippy::version = "1.74.0"]
    pub ZOMBIE_PROCESSES,
    suspicious,
    "not waiting on a spawned child process"
}
declare_lint_pass!(ZombieProcesses => [ZOMBIE_PROCESSES]);

fn emit_lint(cx: &LateContext<'_>, span: Span) {
    span_lint_and_then(
        cx,
        ZOMBIE_PROCESSES,
        span,
        "spawned process is never `wait()`-ed on and leaves behind a zombie process",
        |diag| {
            diag.help("consider calling `.wait()`")
                .note("also see https://doc.rust-lang.org/stable/std/process/struct.Child.html#warning");
        },
    );
}

impl<'tcx> LateLintPass<'tcx> for ZombieProcesses {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(..) | ExprKind::MethodCall(..) = expr.kind
            && let Some(child_adt) = cx.typeck_results().expr_ty(expr).ty_adt_def()
            && match_def_path(cx, child_adt.did(), &paths::CHILD)
        {
            match cx.tcx.hir().get_parent(expr.hir_id) {
                Node::Local(local) if let PatKind::Binding(_, local_id, ..) = local.pat.kind => {

                    // If the `Child` is assigned to a variable, we want to check if the code never calls `.wait()`
                    // on the variable, and lint if not.
                    // This is difficult to do because expressions can be arbitrarily complex
                    // and the variable can "escape" in various ways, e.g. you can take a `&mut` to the variable
                    // and call `.wait()` through that, or pass it to another function...
                    // So instead we do the inverse, checking if all uses are either:
                    // - a field access (`child.{stderr,stdin,stdout}`)
                    // - calling `id` or `kill`
                    // - no use at all (e.g. `let _x = child;`)
                    // - taking a shared reference (`&`), `wait()` can't go through that
                    // Neither of these is sufficient to prevent zombie processes
                    // Doing it like this means more FNs, but FNs are better than FPs.
                    let has_no_wait = for_each_local_use_after_expr(cx, local_id, expr.hir_id, |expr| {
                        match cx.tcx.hir().get_parent(expr.hir_id) {
                            Node::Stmt(Stmt { kind: StmtKind::Semi(_), .. }) => ControlFlow::Continue(()),
                            Node::Expr(expr) if let ExprKind::Field(..) = expr.kind => ControlFlow::Continue(()),
                            Node::Expr(expr) if let ExprKind::AddrOf(_, Mutability::Not, _) = expr.kind => {
                                ControlFlow::Continue(())
                            }
                            Node::Expr(expr)
                                if let Some(fn_did) = fn_def_id(cx, expr)
                                    && match_any_def_paths(cx, fn_did, &[
                                        &paths::CHILD_ID,
                                        &paths::CHILD_KILL,
                                    ]).is_some() =>
                            {
                                ControlFlow::Continue(())
                            }

                            // Conservatively assume that all other kinds of nodes call `.wait()` somehow.
                            _ => ControlFlow::Break(()),
                        }
                    }).is_continue();

                    if has_no_wait {
                        emit_lint(cx, expr.span);
                    }
                },
                Node::Local(local) if let PatKind::Wild = local.pat.kind => {
                    // `let _ = child;`, also dropped immediately without `wait()`ing
                    emit_lint(cx, expr.span);
                }
                Node::Stmt(Stmt { kind: StmtKind::Semi(_), .. }) => {
                    // Immediately dropped. E.g. `std::process::Command::new("echo").spawn().unwrap();`
                    emit_lint(cx, expr.span);
                }
                _ => {}
            }
        }
    }
}
