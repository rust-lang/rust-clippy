use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::{def, def_id, Expr, ExprKind, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;
use std::collections::HashMap;

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
    let idxs: HashMap<&String, usize> = definition
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

impl<'tcx> LateLintPass<'tcx> for SuspiciousArguments {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Call(f, args) = &expr.kind
            && let ExprKind::Path(qp) = &f.kind {
                let hirid = match qp {
                    QPath::Resolved(_, p) => {
                        let last_segment = p.segments.last().unwrap().res.unwrap();

                        match last_segment {
                            def::Res::Def(_, defid) => {
                                if defid.krate != def_id::LOCAL_CRATE {
                                    return;
                                }
                                let local = def_id::LocalDefId {
                                    local_def_index: defid.index,
                                };
                                let hirid = cx.tcx.hir().local_def_id_to_hir_id(local);
                                hirid
                            }
                            _ => return,
                        }
                    }
                    QPath::TypeRelative(_, _) => {
                        let res = cx.typeck_results().qpath_res(qp, f.hir_id);
                        let Some(defid) = res.opt_def_id() else { return };
                        if defid.krate != def_id::LOCAL_CRATE {
                            return;
                        }
                        let local = def_id::LocalDefId {
                            local_def_index: defid.index,
                        };
                        let hirid = cx.tcx.hir().local_def_id_to_hir_id(local);
                        hirid
                    }
                    QPath::LangItem(_, _, _) => { return; }
                };


                let Some(bodyid) = cx.tcx.hir().maybe_body_owned_by(hirid) else { return; };
                let body = cx.tcx.hir().body(bodyid);

                let mut def_args = Vec::new();
                for param in body.params {
                    let PatKind::Binding(_, _, ident, _) = &param.pat.kind else { return };
                    def_args.push((ident.to_string(), ident.span));
                }

                let mut call_args = Vec::new();

                for call_arg in *args {
                    if let ExprKind::Path(qp) = &call_arg.kind
                    && let QPath::Resolved(_, p) = qp
                    && let &[segment] = &p.segments {
                        call_args.push(Some((segment.ident.to_string(), call_arg.span)));
                    } else {
                        call_args.push(None);
                    }
                }

                arguments_are_sus(cx, &def_args, &call_args);
        }
    }
}
