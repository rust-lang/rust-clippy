#![allow(clippy::redundant_clone)]

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::fn_def_id;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Suggests a better pattern of code, when a trait is used explicitly.
    /// ### Why is this bad?
    /// It's poorly readable.
    /// ### Example
    /// ```rust
    /// f32::floor(4.5f32);
    /// ```
    /// Use instead:
    /// ```rust
    /// (4.5f32).floor();
    /// ```
    #[clippy::version = "1.66.0"]
    pub DIRECT_METHOD_CALL,
    complexity,
    "Needlessly using explicit trait"
}

declare_lint_pass!(DirectMethodCall => [DIRECT_METHOD_CALL]);

// 'X::Y(Z) -> Z.Y()' When Z implements X

impl LateLintPass<'_> for DirectMethodCall {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let Some(fnid) = fn_def_id(cx, expr) {
            if let Some(fnsig) = cx.tcx.opt_associated_item(fnid) {
                if fnsig.fn_has_self_parameter &&
            // Discard if it's already a method x.y()
            !cx.typeck_results().is_method_call(expr)
                {
                    let snippet_raw = snippet_with_context(
                        cx,
                        expr.span,
                        expr.span.ctxt(),
                        "..",
                        &mut Applicability::MaybeIncorrect,
                    )
                    .0;
                    let snippet_formatted = format_snippet(&snippet_raw);
                    span_lint_and_sugg(
                        cx,
                        DIRECT_METHOD_CALL,
                        expr.span,
                        "this is poorly readable",
                        "did you mean",
                        snippet_formatted,
                        Applicability::MaybeIncorrect,
                    );
                }
            }
        }
    }
}

fn format_snippet(snippet_raw: &str) -> String {
    // W::X::Y(Z, ...N) = Y.Z(...N)
    let mut segments = snippet_raw.split("::").collect::<Vec<&str>>();
    segments.remove(segments.len() - 2);

    let binding = segments.join("::");
    let no_trait = binding.split('(').collect::<Vec<&str>>();
    let method_name = no_trait[0];
    let mut args = no_trait[1].split(')').collect::<Vec<&str>>()[0].to_owned();
    // Remove whitespace
    args.retain(|c| !c.is_whitespace());

    let args = args.split(',').collect::<Vec<&str>>();
    let to_return = format!("({}).{method_name}({})", args[0], args[1..].join(","));
    to_return
}
