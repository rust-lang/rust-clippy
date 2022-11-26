#![allow(clippy::redundant_clone)]

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::fn_def_id;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LintContext;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};

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

impl_lint_pass!(DirectMethodCall => [DIRECT_METHOD_CALL]);

pub struct DirectMethodCall {
    allowed_explicit_modules: Vec<String>,
}

impl DirectMethodCall {
    #[must_use]
    pub fn new(allowed_explicit_modules: Vec<String>) -> Self {
        Self {
            allowed_explicit_modules,
        }
    }
}

// 'X::Y(Z) -> Z.Y()' When Z implements X

impl LateLintPass<'_> for DirectMethodCall {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if !in_external_macro(cx.sess(), expr.span) {
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
                        let snippet_formatted = format_snippet(&snippet_raw, &self.allowed_explicit_modules);
                        if let Some(snip) = snippet_formatted {
                            span_lint_and_sugg(
                                cx,
                                DIRECT_METHOD_CALL,
                                expr.span,
                                "this is poorly readable",
                                "did you mean",
                                snip,
                                Applicability::MaybeIncorrect,
                            );
                        }
                    }
                }
            }
        }
    }
}

// This is an expensive function.
fn format_snippet(snippet_raw: &str, allowed_explicit_modules: &Vec<String>) -> Option<String> {
    // W::X::Y(Z, ...N) = Y.Z(...N)
    let segments = snippet_raw.split("(").collect::<Vec<&str>>();
    if segments.len() <= 1 {
        return None;
    }
    let binding = segments[1].split(')').collect::<Vec<&str>>();
    let suffixes = binding[binding.len() - 1];
    let mut args: Vec<String> = Vec::new();
    {
        let mut args_len;
        let raw_args = segments[1].split(')').collect::<Vec<&str>>()[0]
            .split(',')
            .collect::<Vec<&str>>();
        // Convert raw_args to String (args);
        for raw_arg in raw_args {
            args.push(raw_arg.to_owned());
            args_len = args.len();
            args[args_len - 1].retain(|c| !c.is_whitespace());
        }
    }

    // Ignore if module name is in conf.allowed_explicit_modules

    let mut ident = segments[0];
    let mut deconstructed_ident = ident.split("::").collect::<Vec<&str>>();
    for &ident in &deconstructed_ident {
        if allowed_explicit_modules.contains(&ident.to_owned()) {
            return None;
        }
    }

    if deconstructed_ident.len() >= 2 {
        // W::X::Y(Z, ...N)
        // 1  2  ---- 3 ----
        // Remove 2
        deconstructed_ident.remove(deconstructed_ident.len() - 2);
    } else {
        return None;
    }

    let binding = deconstructed_ident.join("::");
    ident = &binding;
    // Remove whitespace
    let to_return = format!("({}).{ident}({}){suffixes}", args[0], args[1..].join(","));
    Some(to_return)
}
