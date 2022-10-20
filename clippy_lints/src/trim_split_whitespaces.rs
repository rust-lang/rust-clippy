use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.65.0"]
    pub TRIM_SPLIT_WHITESPACES,
    style,
    "default lint description"
}
declare_lint_pass!(TrimSplitWhitespaces => [TRIM_SPLIT_WHITESPACES]);

impl<'tcx> LateLintPass<'tcx> for TrimSplitWhitespaces {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        let tyckres = cx.typeck_results();

        if let ExprKind::MethodCall(path, [split_recv], split_ws_span) = expr.kind
            && path.ident.name == sym!(split_whitespace)
            && let Some(split_ws_def_id) = tyckres.type_dependent_def_id(expr.hir_id)
        {
            if cx.tcx.is_diagnostic_item(sym::str_split_whitespace, split_ws_def_id) {
                println!("This passed. split_ws_def_id ==> {:?}", split_ws_def_id);
                if let ExprKind::MethodCall(path, [_trim_recv], trim_span) = split_recv.kind
                    && let trim_fn_name @ ("trim" | "trim_start" | "trim_end") = path.ident.name.as_str()
                    && let Some(trim_def_id) = tyckres.type_dependent_def_id(split_recv.hir_id) {
                        if is_one_of_trim_diagnostic_items(cx, trim_def_id) {
                            println!("This passed. trim_def_id ==> {:?}", trim_def_id);
                            println!("YYY {:?} got linted!", expr.span);
                            span_lint_and_sugg(
                                cx,
                                TRIM_SPLIT_WHITESPACES,
                                trim_span.with_hi(split_ws_span.lo()),
                                &format!("found call to `str::{}` before `str::split_whitespace`", trim_fn_name),
                                &format!("remove `{}()`", trim_fn_name),
                                String::new(),
                                Applicability::MachineApplicable,
                            );
                        } else {
                            println!("This didn't pass. trim_def_id ==> {:?}", trim_def_id);
                            println!("XXX {:?} did not linted!", expr.span);
                        }
                    }
            } else {
                println!("This didn't pass. split_ws_def_id ==> {:?}", split_ws_def_id);
                println!("XXX {:?} did not linted!", expr.span);
            }
        }
        println!();
    }
}

fn is_one_of_trim_diagnostic_items(cx: &LateContext<'_>, trim_def_id: DefId) -> bool {
    cx.tcx.is_diagnostic_item(sym::str_trim, trim_def_id)
        || cx.tcx.is_diagnostic_item(sym::str_trim_start, trim_def_id)
        || cx.tcx.is_diagnostic_item(sym::str_trim_end, trim_def_id)
}
