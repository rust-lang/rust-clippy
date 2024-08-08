#![allow(clippy::similar_names)]
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that the caller enables the target features that the callee requires
    ///
    /// ### Why is this bad?
    /// Not enabling target features can cause UB and limits optimization opportunities.
    ///
    /// ### Example
    /// ```no_run
    /// #[target_feature(enable = "avx2")]
    /// unsafe fn f() -> u32 {
    ///     0
    /// }
    ///
    /// fn g() {
    ///     unsafe { f() };
    ///     // g does not enable the target features f requires
    /// }
    /// ```
    #[clippy::version = "1.82.0"]
    pub CALL_MISSING_TARGET_FEATURE,
    suspicious,
    "call requires target features that the surrounding function does not enable"
}

declare_lint_pass!(CallMissingTargetFeature => [CALL_MISSING_TARGET_FEATURE]);

impl<'tcx> LateLintPass<'tcx> for CallMissingTargetFeature {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr<'tcx>) {
        let Some(callee_def_id) = callee_def_id(cx, expr) else {
            return;
        };
        let callee_target_features = &cx.tcx.codegen_fn_attrs(callee_def_id).target_features;

        if callee_target_features.is_empty() {
            return;
        }

        let Some(caller_body_id) = cx.enclosing_body else {
            return;
        };
        let caller_def_id = cx.tcx.hir().body_owner_def_id(caller_body_id);
        let caller_target_features = &cx.tcx.codegen_fn_attrs(caller_def_id).target_features;

        if in_external_macro(cx.tcx.sess, expr.span) {
            return;
        }

        // target features can imply other target features (e.g. avx2 implies sse4.2). We can safely skip
        // implied target features and only warn for the more general missing target feature.
        let missing: Vec<_> = callee_target_features
            .iter()
            .filter_map(|target_feature| {
                if target_feature.implied || caller_target_features.iter().any(|tf| tf.name == target_feature.name) {
                    None
                } else {
                    Some(target_feature.name.as_str())
                }
            })
            .collect();

        if missing.is_empty() {
            return;
        }

        let attr = format!("#[target_feature(enable = \"{}\")]", missing.join(","));

        span_lint_and_then(
            cx,
            CALL_MISSING_TARGET_FEATURE,
            expr.span,
            "this call requires target features that the surrounding function does not enable",
            |diag| {
                diag.span_label(
                    expr.span,
                    "this function call requires target features to be enabled".to_string(),
                );

                let fn_sig = cx.tcx.fn_sig(caller_def_id).skip_binder();

                let mut suggestions = Vec::with_capacity(2);

                let hir::Node::Item(caller_item) = cx.tcx.hir_node_by_def_id(caller_def_id) else {
                    return;
                };

                let Some(indent) = clippy_utils::source::snippet_indent(cx, caller_item.span) else {
                    return;
                };

                let lo_span = caller_item.span.with_hi(caller_item.span.lo());

                match fn_sig.safety() {
                    hir::Safety::Safe => {
                        // the `target_feature` attribute can only be applied to unsafe functions
                        if caller_item.vis_span.is_empty() {
                            suggestions.push((lo_span, format!("{attr}\n{indent}unsafe ")));
                        } else {
                            suggestions.push((lo_span, format!("{attr}\n{indent}")));
                            suggestions.push((caller_item.vis_span.shrink_to_hi(), " unsafe".to_string()));
                        }
                    },
                    hir::Safety::Unsafe => {
                        suggestions.push((lo_span, format!("{attr}\n{indent}")));
                    },
                }

                diag.multipart_suggestion_verbose(
                    "add the missing target features to the surrounding function",
                    suggestions,
                    rustc_errors::Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

fn callee_def_id(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> Option<DefId> {
    match expr.kind {
        hir::ExprKind::Call(path, _) => {
            if let hir::ExprKind::Path(ref qpath) = path.kind
                && let Res::Def(_, did) = cx.qpath_res(qpath, path.hir_id)
            {
                Some(did)
            } else {
                None
            }
        },
        hir::ExprKind::MethodCall(..) => cx.typeck_results().type_dependent_def_id(expr.hir_id),
        _ => None,
    }
}
