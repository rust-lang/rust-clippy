use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{root_macro_call, FormatArgsExpn};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;
use std::fmt::Write as _;
use std::path::Path;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `PathBuf::from(format!(..))` calls.
    ///
    /// ### Why is this bad?
    /// It is not OS-agnostic, and can be harder to read.
    ///
    /// ### Known Problems
    /// `.join()` introduces additional allocations that are not present when `PathBuf::push` is
    /// used instead.
    ///
    /// ### Example
    /// ```rust
    /// use std::path::PathBuf;
    /// let base_path = "/base";
    /// PathBuf::from(format!("{}/foo/bar", base_path));
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::path::Path;
    /// let base_path = "/base";
    /// Path::new(&base_path).join("foo").join("bar");
    /// ```
    #[clippy::version = "1.62.0"]
    pub PATHS_FROM_FORMAT,
    pedantic,
    "builds a `PathBuf` from a format macro"
}

declare_lint_pass!(PathsFromFormat => [PATHS_FROM_FORMAT]);

impl<'tcx> LateLintPass<'tcx> for PathsFromFormat {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(_, [args, ..]) = expr.kind;
            if let ty = cx.typeck_results().expr_ty(expr);
            if is_type_diagnostic_item(cx, ty, sym::PathBuf);
            if !args.is_empty();
            if let Some(macro_call) = root_macro_call(args[0].span);
            if cx.tcx.item_name(macro_call.def_id) == sym::format;
            if let Some(format_args) = FormatArgsExpn::find_nested(cx, &args[0], macro_call.expn);
            then {
                let format_string_parts = format_args.format_string.parts;
                let format_value_args = format_args.args;
                let string_parts: Vec<&str> = format_string_parts
                    .iter()
                    .map(rustc_span::Symbol::as_str)
                    .collect();
                let mut applicability = Applicability::MachineApplicable;
                let real_vars: Vec<Sugg<'_>> = format_value_args
                    .iter()
                    .map(|x| Sugg::hir_with_applicability(cx, x.param.value, "..", &mut applicability))
                    .collect();
                let mut paths_zip = string_parts.iter().take(real_vars.len()).zip(real_vars.clone());
                let mut sugg = String::new();
                if let Some((part, arg)) = paths_zip.next() {
                    if is_valid_use_case(string_parts.first().unwrap_or(&""), string_parts.get(1).unwrap_or(&"")) {
                        return;
                    }
                    if part.is_empty() {
                        sugg = format!("Path::new(&{arg})");
                    }
                    else {
                        push_comps(&mut sugg, part);
                        let _ = write!(sugg, ".join(&{arg})");
                    }
                }
                for n in 1..real_vars.len() {
                    if let Some((part, arg)) = paths_zip.next() {
                        if is_valid_use_case(string_parts.get(n).unwrap_or(&""), string_parts.get(n+1).unwrap_or(&"")) {
                            return;
                        }
                        else if n < real_vars.len() {
                            push_comps(&mut sugg, part);
                            let _ = write!(sugg, ".join(&{arg})");
                        }
                        else {
                            sugg = format!("{sugg}.join(&{arg})");
                        }
                    }
                }
                if real_vars.len() < string_parts.len() {
                    push_comps(&mut sugg, string_parts[real_vars.len()]);
                }
                span_lint_and_sugg(
                    cx,
                    PATHS_FROM_FORMAT,
                    expr.span,
                    "`format!(..)` used to form `PathBuf`",
                    "consider using `Path::new()` and `.join()` to make it OS-agnostic and improve code readability",
                    sugg,
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

fn push_comps(string: &mut String, path: &str) {
    let mut path = path.to_string();
    if !string.is_empty() {
        path = path.trim_start_matches(|c| c == '\\' || c == '/').to_string();
    }
    for n in Path::new(&path).components() {
        let mut x = n.as_os_str().to_string_lossy().to_string();
        if string.is_empty() {
            let _ = write!(string, "Path::new(\"{x}\")");
        } else {
            x = x.trim_end_matches(|c| c == '/' || c == '\\').to_string();
            let _ = write!(string, ".join(\"{x}\")");
        }
    }
}

fn is_valid_use_case(string: &str, string2: &str) -> bool {
    !(string.is_empty() || string.ends_with('/') || string.ends_with('\\'))
        || !(string2.is_empty() || string2.starts_with('/') || string2.starts_with('\\'))
}
