use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::std_or_core;
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty spin loops
    ///
    /// ### Why is this bad?
    /// The loop body should have something like `thread::park()` or at least
    /// `std::hint::spin_loop()` to avoid needlessly burning cycles and conserve
    /// energy. Perhaps even better use an actual lock, if possible.
    ///
    /// ### Known problems
    /// This lint doesn't currently trigger on `while let` or
    /// `loop { match .. { .. } }` loops, which would be considered idiomatic in
    /// combination with e.g. `AtomicBool::compare_exchange_weak`.
    ///
    /// ### Example
    ///
    /// ```ignore
    /// use core::sync::atomic::{AtomicBool, Ordering};
    /// let b = AtomicBool::new(true);
    /// // give a ref to `b` to another thread,wait for it to become false
    /// while b.load(Ordering::Acquire) {};
    /// ```
    /// Use instead:
    /// ```rust,no_run
    ///# use core::sync::atomic::{AtomicBool, Ordering};
    ///# let b = AtomicBool::new(true);
    /// while b.load(Ordering::Acquire) {
    ///     std::hint::spin_loop()
    /// }
    /// ```
    #[clippy::version = "1.61.0"]
    pub MISSING_SPIN_LOOP,
    perf,
    "An empty busy waiting loop"
}

fn unpack_cond<'tcx>(cond: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match &cond.kind {
        ExprKind::Block(
            Block {
                stmts: [],
                expr: Some(e),
                ..
            },
            _,
        )
        | ExprKind::Unary(_, e) => unpack_cond(e),
        ExprKind::Binary(_, l, r) => {
            let l = unpack_cond(l);
            if let ExprKind::MethodCall(..) = l.kind {
                l
            } else {
                unpack_cond(r)
            }
        },
        _ => cond,
    }
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, cond: &'tcx Expr<'_>, body: &'tcx Expr<'_>) {
    if let ExprKind::Block(
        Block {
            stmts: [], expr: None, ..
        },
        _,
    ) = body.kind
        && let ExprKind::MethodCall(method, callee, ..) = unpack_cond(cond).kind
        && [sym::load, sym::compare_exchange, sym::compare_exchange_weak].contains(&method.ident.name)
        && let ty::Adt(def, _args) = cx.typeck_results().expr_ty(callee).kind()
        && cx.tcx.is_diagnostic_item(sym::AtomicBool, def.did())
        && let Some(std_or_core) = std_or_core(cx)
    {
        span_lint_and_sugg(
            cx,
            MISSING_SPIN_LOOP,
            body.span,
            "busy-waiting loop should at least have a spin loop hint",
            "try",
            format!("{{ {std_or_core}::hint::spin_loop() }}"),
            Applicability::MachineApplicable,
        );
    }
}
