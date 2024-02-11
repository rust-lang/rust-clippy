use clippy_config::msrvs::Msrv;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::fn_has_unsatisfiable_preds;
use clippy_utils::qualify_min_const_fn::is_min_const_fn;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{intravisit, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
use rustc_span::sym::thread_local_macro;

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
    ///     static BUF: String = String::new();
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// thread_local! {
    ///     static BUF: String = const { String::new() };
    /// }
    /// ```
    #[clippy::version = "1.76.0"]
    pub THREAD_LOCAL_INITIALIZER_CAN_BE_MADE_CONST,
    perf,
    "suggest using `const` in `thread_local!` macro"
}

pub struct ThreadLocalInitializerCanBeMadeConst {
    msrv: Msrv,
}

impl ThreadLocalInitializerCanBeMadeConst {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl_lint_pass!(ThreadLocalInitializerCanBeMadeConst => [THREAD_LOCAL_INITIALIZER_CAN_BE_MADE_CONST]);

#[inline]
fn is_thread_local_macro(
    cx: &LateContext<'_>,
    fn_kind: rustc_hir::intravisit::FnKind<'_>,
    span: rustc_span::Span,
) -> bool {
    if in_external_macro(cx.sess(), span)
        && let Some(callee) = span.source_callee()
        && let Some(macro_def_id) = callee.macro_def_id
        && cx.tcx.is_diagnostic_item(thread_local_macro, macro_def_id)
        && let intravisit::FnKind::ItemFn(..) = fn_kind
    {
        return true;
    }
    false
}

#[inline]
fn initializer_fn_is_const(cx: &LateContext<'_>, defid: rustc_span::def_id::DefId) -> bool {
    // check that the initializer function is not already `const`.
    // this done for `os_local` fallback as that implementation includes an `__init()`
    // function regardless of whether the `const` keyword is used.
    // For const-qualified `thread_local!`s, the `__init` function is a const fn.
    // For non-const-qualified `thread_local!`s, the `__init` function is not a const fn.
    // we are going to check that we are in a non-const-qualified `thread_local!` and
    // then that the `__init` contents are const-qualifiable. If they are,
    // we can suggest to make the `__init` function const.
    cx.tcx.is_const_fn(defid)
}

#[inline]
fn initializer_can_be_made_const(cx: &LateContext<'_>, defid: rustc_span::def_id::DefId, msrv: &Msrv) -> bool {
    // Building MIR for `fn`s with unsatisfiable preds results in ICE.
    if !fn_has_unsatisfiable_preds(cx, defid)
        && let mir = cx.tcx.optimized_mir(defid)
        && let Ok(()) = is_min_const_fn(cx.tcx, mir, msrv)
    {
        return true;
    }
    false
}

impl<'tcx> LateLintPass<'tcx> for ThreadLocalInitializerCanBeMadeConst {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        fn_kind: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        span: rustc_span::Span,
        local_defid: rustc_span::def_id::LocalDefId,
    ) {
        let defid = local_defid.to_def_id();
        if is_thread_local_macro(cx, fn_kind, span)
            && !initializer_fn_is_const(cx, defid)
            && initializer_can_be_made_const(cx, defid, &self.msrv)
            // this is the `__init` function emitted by the `thread_local!` macro
            // when the `const` keyword is not used. We avoid checking the `__init` directly
            // as that is not a public API.
            // we know that the function is const-qualifiable, so now we need only to get the
            // initializer expression to span-lint it.
            && let ExprKind::Block(block, _) = body.value.kind
            && let Some(ret_expr) = block.expr
            && let initializer_snippet = snippet(cx, ret_expr.span, "thread_local! { ... }")
            && initializer_snippet != "thread_local! { ... }"
        {
            span_lint_and_sugg(
                cx,
                THREAD_LOCAL_INITIALIZER_CAN_BE_MADE_CONST,
                ret_expr.span,
                "initializer for `thread_local` value can be made `const`",
                "replace with",
                format!("const {{ {initializer_snippet} }}"),
                Applicability::MachineApplicable,
            );
        }
    }

    extract_msrv_attr!(LateContext);
}
