use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::visitors::for_each_local_use_after_expr;
use clippy_utils::{fn_def_id, get_enclosing_block, match_any_def_paths, match_def_path, paths};
use rustc_ast::Mutability;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{walk_expr, Visitor};
use rustc_hir::{Expr, ExprKind, HirId, Local, Node, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;
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

impl<'tcx> LateLintPass<'tcx> for ZombieProcesses {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(..) | ExprKind::MethodCall(..) = expr.kind
            && let Some(child_adt) = cx.typeck_results().expr_ty(expr).ty_adt_def()
            && match_def_path(cx, child_adt.did(), &paths::CHILD)
        {
            match cx.tcx.parent_hir_node(expr.hir_id) {
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
                    // None of these are sufficient to prevent zombie processes
                    // Doing it like this means more FNs, but FNs are better than FPs.
                    let has_no_wait = for_each_local_use_after_expr(cx, local_id, expr.hir_id, |expr| {
                        match cx.tcx.parent_hir_node(expr.hir_id) {
                            Node::Stmt(Stmt {
                                kind: StmtKind::Semi(_),
                                ..
                            }) => ControlFlow::Continue(()),
                            Node::Expr(expr) if let ExprKind::Field(..) = expr.kind => ControlFlow::Continue(()),
                            Node::Expr(expr) if let ExprKind::AddrOf(_, Mutability::Not, _) = expr.kind => {
                                ControlFlow::Continue(())
                            },
                            Node::Expr(expr)
                                if let Some(fn_did) = fn_def_id(cx, expr)
                                    && match_any_def_paths(cx, fn_did, &[&paths::CHILD_ID, &paths::CHILD_KILL])
                                        .is_some() =>
                            {
                                ControlFlow::Continue(())
                            },

                            // Conservatively assume that all other kinds of nodes call `.wait()` somehow.
                            _ => ControlFlow::Break(()),
                        }
                    })
                    .is_continue();

                    // If it does have a `wait()` call, we're done. Don't lint.
                    if !has_no_wait {
                        return;
                    }

                    // Don't emit a suggestion since the binding is used later
                    check(cx, expr, local.hir_id, false);
                },
                Node::Local(&Local { pat, hir_id, .. }) if let PatKind::Wild = pat.kind => {
                    // `let _ = child;`, also dropped immediately without `wait()`ing
                    check(cx, expr, hir_id, true);
                },
                Node::Stmt(&Stmt {
                    kind: StmtKind::Semi(_),
                    hir_id,
                    ..
                }) => {
                    // Immediately dropped. E.g. `std::process::Command::new("echo").spawn().unwrap();`
                    check(cx, expr, hir_id, true);
                },
                _ => {},
            }
        }
    }
}

/// This function has shared logic between the different kinds of nodes that can trigger the lint.
///
/// In particular, `let <binding> = <expr that spawns child>;` requires some custom additional logic
/// such as checking that the binding is not used in certain ways, which isn't necessary for
/// `let _ = <expr that spawns child>;`.
///
/// This checks if the program doesn't unconditionally exit after the spawn expression and that it
/// isn't the last statement of the program.
fn check<'tcx>(cx: &LateContext<'tcx>, spawn_expr: &'tcx Expr<'tcx>, node_id: HirId, emit_suggestion: bool) {
    let Some(block) = get_enclosing_block(cx, spawn_expr.hir_id) else {
        return;
    };

    let mut vis = ExitPointFinder {
        cx,
        state: ExitPointState::LookingForSpawnExpr(spawn_expr.hir_id),
    };
    vis.visit_block(block);

    // Visitor found an unconditional `exit()` call, so don't lint.
    if let ExitPointState::ExitFound = vis.state {
        return;
    }

    // This might be the last effective node of the program (main function).
    // There's no need to lint in that case either, as this is basically equivalent to calling `exit()`
    if is_last_node_in_main(cx, node_id) {
        return;
    }

    span_lint_and_then(
        cx,
        ZOMBIE_PROCESSES,
        spawn_expr.span,
        "spawned process is never `wait()`ed on",
        |diag| {
            if emit_suggestion {
                diag.span_suggestion(
                    spawn_expr.span.shrink_to_hi(),
                    "try",
                    ".wait()",
                    Applicability::MaybeIncorrect,
                );
            } else {
                diag.note("consider calling `.wait()`");
            }

            diag.note("not doing so might leave behind zombie processes")
                .note("see https://doc.rust-lang.org/stable/std/process/struct.Child.html#warning");
        },
    );
}

/// The hir id id may either correspond to a `Local` or `Stmt`, depending on how we got here.
/// This function gets the enclosing function, checks if it's `main` and if so,
/// check if the last statement modulo blocks is the given id.
fn is_last_node_in_main(cx: &LateContext<'_>, id: HirId) -> bool {
    let hir = cx.tcx.hir();
    let body_owner = hir.enclosing_body_owner(id);
    let enclosing_body = hir.body(hir.body_owned_by(body_owner));

    if let Some((main_def_id, _)) = cx.tcx.entry_fn(())
        && main_def_id == body_owner.to_def_id()
        && let ExprKind::Block(block, _) = &enclosing_body.value.peel_blocks().kind
        && let [.., stmt] = block.stmts
    {
        matches!(stmt.kind, StmtKind::Let(local) if local.hir_id == id)
            || matches!(stmt.kind, StmtKind::Semi(..) if stmt.hir_id == id)
    } else {
        false
    }
}

/// Checks if the given expression exits the process.
fn is_exit_expression(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    fn_def_id(cx, expr).is_some_and(|fn_did| {
        cx.tcx.is_diagnostic_item(sym::process_exit, fn_did) || match_def_path(cx, fn_did, &paths::ABORT)
    })
}

#[derive(Debug)]
enum ExitPointState {
    /// Still walking up to the expression that initiated the visitor.
    LookingForSpawnExpr(HirId),
    /// We're inside of a control flow construct (e.g. `if`, `match`, `loop`)
    /// Within this, we shouldn't accept any `exit()` calls in here, but we can leave all of these
    /// constructs later and still continue looking for an `exit()` call afterwards. Example:
    /// ```ignore
    /// Command::new("").spawn().unwrap();
    ///
    /// if true {                // depth=1
    ///     if true {            // depth=2
    ///         match () {       // depth=3
    ///             () => loop { // depth=4
    ///
    ///                 std::process::exit();
    ///                 ^^^^^^^^^^^^^^^^^^^^^ conditional exit call, ignored
    ///
    ///             }           // depth=3
    ///         }               // depth=2
    ///     }                   // depth=1
    /// }                       // depth=0
    ///
    /// std::process::exit();
    /// ^^^^^^^^^^^^^^^^^^^^^ this exit call is accepted because we're now unconditionally calling it
    /// ```
    /// We can only get into this state from `NoExit`.
    InControlFlow { depth: u32 },
    /// No exit call found yet, but looking for one.
    NoExit,
    /// Found an expression that exits the process.
    ExitFound,
}

fn expr_enters_control_flow(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Loop(..))
}

struct ExitPointFinder<'a, 'tcx> {
    state: ExitPointState,
    cx: &'a LateContext<'tcx>,
}

impl<'a, 'tcx> Visitor<'tcx> for ExitPointFinder<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match self.state {
            ExitPointState::LookingForSpawnExpr(id) if expr.hir_id == id => {
                self.state = ExitPointState::NoExit;
                walk_expr(self, expr);
            },
            ExitPointState::NoExit if expr_enters_control_flow(expr) => {
                self.state = ExitPointState::InControlFlow { depth: 1 };
                walk_expr(self, expr);
                if let ExitPointState::InControlFlow { .. } = self.state {
                    self.state = ExitPointState::NoExit;
                }
            },
            ExitPointState::NoExit if is_exit_expression(self.cx, expr) => self.state = ExitPointState::ExitFound,
            ExitPointState::InControlFlow { ref mut depth } if expr_enters_control_flow(expr) => {
                *depth += 1;
                walk_expr(self, expr);
                match self.state {
                    ExitPointState::InControlFlow { depth: 1 } => self.state = ExitPointState::NoExit,
                    ExitPointState::InControlFlow { ref mut depth } => *depth -= 1,
                    _ => {},
                }
            },
            _ => {},
        }
    }
}
