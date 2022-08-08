use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::path_res;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for calls to a function where the parameters look swapped
    ///
    /// ### Why is this bad?
    ///
    /// This likely indicates an error, where the arguments were reversed.
    ///
    /// ### Example
    /// ```rust
    /// fn resize(width: usize, height: usize) {}
    ///
    /// let height = 100;
    /// let width = 200;
    /// resize(height, width);
    /// ```
    /// Use instead:
    /// ```rust
    /// fn resize(width: usize, height: usize) {}
    ///
    /// let height = 100;
    /// let width = 200;
    /// resize(width, height);
    /// ```
    #[clippy::version = "1.64.0"]
    pub SUSPICIOUS_ARGUMENTS,
    suspicious,
    "function call with probably swapped arguments"
}
declare_lint_pass!(SuspiciousArguments => [SUSPICIOUS_ARGUMENTS]);

fn arguments_are_sus(cx: &LateContext<'_>, definition: &[(String, Span)], call: &[Option<(String, Span)>]) {
    let idxs: FxHashMap<&String, usize> = definition
        .iter()
        .enumerate()
        .map(|(idx, (item, _))| (item, idx))
        .collect();

    for (call_idx, arg_and_span) in call.iter().enumerate() {
        if let Some((arg, call_span)) = arg_and_span {
            if let Some(&def_idx) = idxs.get(arg) {
                if call_idx != def_idx {
                    if let Some((reverse_call, reverse_call_span)) = &call[def_idx] {
                        let def_for_call = &definition[call_idx];
                        if reverse_call == &def_for_call.0 {
                            // This is technically being called twice, but it's being
                            // deduplicated?
                            span_lint_and_then(
                                cx,
                                SUSPICIOUS_ARGUMENTS,
                                vec![*call_span, *reverse_call_span],
                                "these arguments are possibly swapped",
                                |diag| {
                                    let second_span = definition[def_idx].1;
                                    diag.span_note(vec![def_for_call.1, second_span], "the arguments are defined here");
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Try and guess an ident for `expr`. If they give something like `my_struct.height`
/// or `height()`, use that.
fn guess_ident(expr: &Expr<'_>) -> Option<(String, Span)> {
    match &expr.kind {
        ExprKind::Path(qp) => {
            if let QPath::Resolved(_, p) = qp && let Some(segment) = p.segments.last() {
                return Some((segment.ident.to_string(), segment.ident.span));
            }
        }
        ExprKind::Field(_, ident) => {
            return Some((ident.to_string(), ident.span));
        }
        ExprKind::Call(func, _) => {
            return guess_ident(func);
        }
        _ => {},
    }

    None
}

impl<'tcx> LateLintPass<'tcx> for SuspiciousArguments {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Call(f, args) = expr.kind
            && let Some(def_id) = path_res(cx, f).opt_def_id() {

            // fn_arg_names will ICE on a tuple struct being constructed.
            //
            // We can't usefully lint on this anyways, tuple struct constructors have no parameter
            // names.
            if cx.tcx.is_constructor(def_id) {
                return;
            }

            // fn_arg_names will ICE on a variadic function.
            //
            // We *could* usefully lint on this (and it seems to work for *foreign* items, just not
            // local ones?)
            if cx.tcx.fn_sig(def_id).c_variadic() {
                return;
            }

            let mut def_args = Vec::new();
            for ident in cx.tcx.fn_arg_names(def_id) {
                def_args.push((ident.to_string(), ident.span));
            }

            let mut call_args = Vec::new();

            for call_arg in args {
                call_args.push(guess_ident(call_arg));
            }

            if def_args.len() != call_args.len() {
                let def_args_query = cx.tcx.fn_arg_names(def_id);
                rustc_middle::span_bug!(expr.span, "{def_args_query:?}\n{args:#?}");
            }

            arguments_are_sus(cx, &def_args, &call_args);
        }
    }
}
