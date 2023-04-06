use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{method_chain_args, return_ty};
use core::ops::ControlFlow;
use if_chain::if_chain;
use rustc_hir as hir;
use rustc_hir::ImplItemKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions of type `Result` that contain `expect()` or `unwrap()`
    ///
    /// ### Why is this bad?
    /// These functions promote recoverable errors to non-recoverable errors which may be undesirable in code bases which wish to avoid panics.
    ///
    /// ### Known problems
    /// This can cause false positives in functions that handle both recoverable and non recoverable errors.
    ///
    /// ### Example
    /// Before:
    /// ```rust
    /// fn divisible_by_3(i_str: String) -> Result<(), String> {
    ///     let i = i_str
    ///         .parse::<i32>()
    ///         .expect("cannot divide the input by three");
    ///
    ///     if i % 3 != 0 {
    ///         Err("Number is not divisible by 3")?
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// After:
    /// ```rust
    /// fn divisible_by_3(i_str: String) -> Result<(), String> {
    ///     let i = i_str
    ///         .parse::<i32>()
    ///         .map_err(|e| format!("cannot divide the input by three: {}", e))?;
    ///
    ///     if i % 3 != 0 {
    ///         Err("Number is not divisible by 3")?
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.48.0"]
    pub UNWRAP_IN_RESULT,
    restriction,
    "functions of type `Result<..>` or `Option`<...> that contain `expect()` or `unwrap()`"
}

declare_lint_pass!(UnwrapInResult=> [UNWRAP_IN_RESULT]);

impl<'tcx> LateLintPass<'tcx> for UnwrapInResult {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx hir::ImplItem<'_>) {
        if_chain! {
            // first check if it's a method or function
            if let hir::ImplItemKind::Fn(ref _signature, _) = impl_item.kind;
            // checking if its return type is `result` or `option`
            if is_type_diagnostic_item(cx, return_ty(cx, impl_item.owner_id), sym::Result)
                || is_type_diagnostic_item(cx, return_ty(cx, impl_item.owner_id), sym::Option);
            then {
                lint_impl_body(cx, impl_item.span, impl_item);
            }
        }
    }
}

fn lint_impl_body<'tcx>(cx: &LateContext<'tcx>, impl_span: Span, impl_item: &'tcx hir::ImplItem<'_>) {
    if let ImplItemKind::Fn(_, body_id) = impl_item.kind {
        let body = cx.tcx.hir().body(body_id);
        let typeck = cx.tcx.typeck(impl_item.owner_id.def_id);
        let mut result = Vec::new();
        let _: Option<!> = for_each_expr(body.value, |e| {
            // check for `expect`
            if let Some(arglists) = method_chain_args(e, &["expect"]) {
                let receiver_ty = typeck.expr_ty(arglists[0].0).peel_refs();
                if is_type_diagnostic_item(cx, receiver_ty, sym::Option)
                    || is_type_diagnostic_item(cx, receiver_ty, sym::Result)
                {
                    result.push(e.span);
                }
            }

            // check for `unwrap`
            if let Some(arglists) = method_chain_args(e, &["unwrap"]) {
                let receiver_ty = typeck.expr_ty(arglists[0].0).peel_refs();
                if is_type_diagnostic_item(cx, receiver_ty, sym::Option)
                    || is_type_diagnostic_item(cx, receiver_ty, sym::Result)
                {
                    result.push(e.span);
                }
            }

            ControlFlow::Continue(())
        });

        // if we've found one, lint
        if !result.is_empty() {
            span_lint_and_then(
                cx,
                UNWRAP_IN_RESULT,
                impl_span,
                "used unwrap or expect in a function that returns result or option",
                move |diag| {
                    diag.help("unwrap and expect should not be used in a function that returns result or option");
                    diag.span_note(result, "potential non-recoverable error(s)");
                },
            );
        }
    }
}
