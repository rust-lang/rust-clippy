use clippy_config::msrvs::Msrv;
use clippy_utils::consts::{constant, constant_full_int, constant_with_source};
use clippy_utils::diagnostics::{span_lint, span_lint_hir};
use clippy_utils::qualify_min_const_fn::is_min_const_fn;
use clippy_utils::source::{snippet, snippet_with_context};
use clippy_utils::{fn_has_unsatisfiable_preds, is_from_proc_macro, is_lang_item_or_ctor};
use rustc_ast::visit::FnKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::mir::tcx;
use rustc_middle::ty::layout::HasTyCtxt;
use rustc_middle::ty::{self};
use rustc_span::sym::thread_local_macro;
use rustc_session::impl_lint_pass;


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


pub struct SuggestConstThreadLocal {
    msrv: Msrv,
}

impl SuggestConstThreadLocal {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(SuggestConstThreadLocal => [SUGGEST_CONST_THREAD_LOCAL]);


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

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        span: rustc_span::Span,
        defid: rustc_span::def_id::LocalDefId,
    ) {
        if in_external_macro(cx.sess(), span)
            && let Some(callee) = span.source_callee()
            && let Some(macro_def_id) = callee.macro_def_id
            && cx.tcx.is_diagnostic_item(thread_local_macro, macro_def_id)
            && let intravisit::FnKind::ItemFn(ident, _, _) = fn_kind
            
            // we are matching only against the `__init` function emitted by the `thread_local!` macro
            // when the `const` keyword is not used.
            && ident.as_str() == "__init"

            // Building MIR for `fn`s with unsatisfiable preds results in ICE.
            && !fn_has_unsatisfiable_preds(cx, defid.to_def_id())
            && let mir = cx.tcx.optimized_mir(defid.to_def_id())
            && let Ok(_) = is_min_const_fn(cx.tcx, mir, &self.msrv)
        {
            // let s: (std::borrow::Cow<'_, str>, bool) = snippet_with_context(cx, body.value.span, outer, default, );
            // let s = snippet(cx, body.value.span, "thread_local! { ... }");
            // dbg!(s);
            span_lint(cx, SUGGEST_CONST_THREAD_LOCAL, body.value.span, "Consider using `const` in `thread_local!` macro");
        }
    }
}
