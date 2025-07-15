use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_in_panic_handler, is_no_std_crate, sym};
use rustc_hir::{Block, Expr, ItemKind, Node};
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty `loop` expressions.
    ///
    /// ### Why is this bad?
    /// These busy loops burn CPU cycles without doing
    /// anything. It is _almost always_ a better idea to `panic!` than to have
    /// a busy loop.
    ///
    /// If panicking isn't possible, think of the environment and either:
    ///   - block on something
    ///   - sleep the thread for some microseconds
    ///   - yield or pause the thread
    ///
    /// For `std` targets, this can be done with
    /// [`std::thread::sleep`](https://doc.rust-lang.org/std/thread/fn.sleep.html)
    /// or [`std::thread::yield_now`](https://doc.rust-lang.org/std/thread/fn.yield_now.html).
    ///
    /// For `no_std` targets, doing this is more complicated, especially because
    /// `#[panic_handler]`s can't panic. To stop/pause the thread, you will
    /// probably need to invoke some target-specific intrinsic. Examples include:
    ///   - [`x86_64::instructions::hlt`](https://docs.rs/x86_64/0.12.2/x86_64/instructions/fn.hlt.html)
    ///   - [`cortex_m::asm::wfi`](https://docs.rs/cortex-m/0.6.3/cortex_m/asm/fn.wfi.html)
    ///
    /// ### Example
    /// ```no_run
    /// loop {}
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub EMPTY_LOOP,
    suspicious,
    "empty `loop {}`, which should block or sleep"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, loop_block: &Block<'_>) {
    let parent_hir_id = cx.tcx.parent_hir_id(expr.hir_id);
    if let Node::Item(parent_node) = cx.tcx.hir_node(parent_hir_id)
        && matches!(parent_node.kind, ItemKind::Fn { .. })
        && let attrs = cx.tcx.hir_attrs(parent_hir_id)
        && attrs.iter().any(|attr| attr.has_name(sym::rustc_intrinsic))
    {
        // Intrinsic functions are expanded into an empty loop when lowering the AST
        // to simplify the job of later passes which might expect any function to have a body.
        return;
    }

    if loop_block.stmts.is_empty() && loop_block.expr.is_none() && !is_in_panic_handler(cx, expr) {
        let msg = "empty `loop {}` wastes CPU cycles";
        let help = if is_no_std_crate(cx) {
            "you should either use `panic!()` or add a call pausing or sleeping the thread to the loop body"
        } else {
            "you should either use `panic!()` or add `std::thread::sleep(..);` to the loop body"
        };
        span_lint_and_help(cx, EMPTY_LOOP, expr.span, msg, None, help);
    }
}
