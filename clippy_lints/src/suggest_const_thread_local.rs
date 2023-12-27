use clippy_utils::consts::{constant_full_int, constant_with_source, constant};
use clippy_utils::{is_from_proc_macro, is_lang_item_or_ctor};
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::mir::tcx;
use rustc_middle::ty::layout::HasTyCtxt;
use rustc_middle::ty::{self};
use rustc_session::declare_lint_pass;
use rustc_span::sym::{thread_local_macro};

declare_clippy_lint! {
    /// ### What it does
    /// Suggests to use `const` in `thread_local!` macro if possible.
    /// ### Why is this bad?
    ///
    /// The `thread_local!` macro wraps static declarations and makes them thread-local.
    /// It supports using a `const` keyword that may be used for declarations that can
    /// be evaluated as a constant expression. This can enable a more efficient thread
    /// local implementation that can avoid lazy initialization. For types that do not
    /// need to be dropped, this can enable an even more efficient implementation that
    /// does not need to track any additional state.
    ///
    /// https://doc.rust-lang.org/std/macro.thread_local.html
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// thread_local! {
    ///     static BUF: RefCell<String> = RefCell::new(String::new());
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// thread_local! {
    ///     static BUF: RefCell<String> = const { RefCell::new(String::new()) };
    /// }
    /// ```
    #[clippy::version = "1.76.0"]
    pub SUGGEST_CONST_THREAD_LOCAL,
    perf,
    "suggest using `const` in `thread_local!` macro"
}

declare_lint_pass!(SuggestConstThreadLocal => [SUGGEST_CONST_THREAD_LOCAL]);

impl<'tcx> LateLintPass<'tcx> for SuggestConstThreadLocal {
    /// The goal is to:
    /// 1. Find all `thread_local!` macro invocations.
    /// 2. Check if the init stmt is using `const` keyword.
    /// 3. If the stmt is not using a `const` keyword, check if we can suggest to use it.
    /// 4. If we can suggest to use `const`, lint it.
    /// 
    /// If a const is used, then the macro expansion does not perform any lazy initialization
    /// and therefore an init function is not needed. We can then match on the existence of
    /// the init function to determine if the const keyword was used.
    /// 
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx rustc_hir::Stmt<'tcx>) {
        if in_external_macro(cx.sess(), stmt.span)
            && let Some(callee) =stmt.span.source_callee()
            && let Some(defid) = callee.macro_def_id
            && cx.tcx.is_diagnostic_item(thread_local_macro, defid)
            && let StmtKind::Item(item) = &stmt.kind
            && let item = cx.tcx.hir().item(*item)
            && let ItemKind::Fn(fn_sig, _, body_id) = item.kind

            // These two cause ICEs :(
            //// && let Some(const_value) = constant(cx, cx.typeck_results(), body.value)
            //// && let Some(_) = constant_with_source(cx, cx.typeck_results(), body.value)
        {
            let body = cx.tcx.hir().body(body_id);
            dbg!(item);
            dbg!(fn_sig);
            dbg!(body);
            dbg!("Found thread_local macro");
        }
    }

}
