#![allow(clippy::similar_names)]
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::fn_has_unsatisfiable_clauses;
use rustc_hir as hir;
use rustc_hir::intravisit::FnKind;
use rustc_hir::def::Res;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_index::bit_set::DenseBitSet;
use rustc_lint::{LateContext, LateLintPass, LintContext as _, declare_lint_pass};
use rustc_middle::middle::codegen_fn_attrs::TargetFeatureKind;
use rustc_middle::mir::{START_BLOCK, TerminatorKind};
use rustc_span::Span;

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
    #[clippy::version = "1.97.0"]
    pub CALL_MISSING_TARGET_FEATURE,
    restriction,
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
        let caller_def_id = cx.tcx.hir_body_owner_def_id(caller_body_id);
        let caller_target_features = &cx.tcx.codegen_fn_attrs(caller_def_id).target_features;

        if expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        // target features can imply other target features (e.g. avx2 implies sse4.2). We can safely skip
        // implied target features and only warn for the more general missing target feature.
        let missing: Vec<_> = callee_target_features
            .iter()
            .filter_map(|target_feature| {
                if target_feature.kind == TargetFeatureKind::Implied
                    || caller_target_features.iter().any(|tf| tf.name == target_feature.name)
                {
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

                let lo_span = caller_item.span.shrink_to_lo();

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

                diag.multipart_suggestion(
                    "add the missing target features to the surrounding function",
                    suggestions,
                    rustc_errors::Applicability::MaybeIncorrect,
                );
            },
        );
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        _: &'tcx hir::Body<'tcx>,
        _: Span,
        local_def_id: LocalDefId,
    ) {
        let caller_def_id = local_def_id.to_def_id();

        if fn_has_unsatisfiable_clauses(cx, caller_def_id) {
            return;
        }

        let body = cx.tcx.optimized_mir(caller_def_id);
        let mut seen_blocks = DenseBitSet::new_empty(body.basic_blocks.len());
        seen_blocks.insert(START_BLOCK);
        let mut outstanding_blocks = 1;

        for &bb in body.basic_blocks.reverse_postorder() {
            if !seen_blocks.contains(bb) {
                // this will skip unwind blocks
                continue;
            }
            outstanding_blocks -= 1;
            let bb = &body.basic_blocks[bb];
            if outstanding_blocks == 0
                && let TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. } = &bb.terminator().kind
                && let Some((callee, _)) = func.const_fn_def()
            {
                // Check target features
                let callee_features = &cx.tcx.codegen_fn_attrs(callee).target_features;
                let caller_features = &cx.tcx.body_codegen_attrs(caller_def_id).target_features;

                // callee must be a subset of caller for this to be safe
                let is_safe = cx.tcx.is_target_feature_call_safe(callee_features, caller_features);
                if !is_safe {
                    let missing: Vec<_> = callee_features
                        .iter()
                        .filter(|feature| {
                            feature.kind != TargetFeatureKind::Implied
                                && !caller_features.iter().any(|tf| tf.name == feature.name)
                        })
                        .map(|feature| feature.name.as_str())
                        .collect();

                    if !missing.is_empty() {
                        let span = bb.terminator().source_info.span;
                        let attr = format!("#[target_feature(enable = \"{}\")]", missing.join(","));

                        span_lint_and_then(
                            cx,
                            CALL_MISSING_TARGET_FEATURE,
                            span,
                            "this call requires target features that the surrounding function does not enable",
                            |diag| {
                                diag.span_label(span, "this function call requires target features to be enabled");

                                let fn_sig = cx.tcx.fn_sig(caller_def_id).skip_binder();

                                let mut suggestions = Vec::with_capacity(2);

                                let hir::Node::Item(caller_item) = cx.tcx.hir_node_by_def_id(local_def_id) else {
                                    return;
                                };

                                let Some(indent) = clippy_utils::source::snippet_indent(cx, caller_item.span) else {
                                    return;
                                };

                                let lo_span = caller_item.span.shrink_to_lo();

                                match fn_sig.safety() {
                                    hir::Safety::Safe => {
                                        // the `target_feature` attribute can only be applied to unsafe functions
                                        if caller_item.vis_span.is_empty() {
                                            suggestions.push((lo_span, format!("{attr}\n{indent}unsafe ")));
                                        } else {
                                            suggestions.push((lo_span, format!("{attr}\n{indent}")));
                                            suggestions
                                                .push((caller_item.vis_span.shrink_to_hi(), " unsafe".to_string()));
                                        }
                                    },
                                    hir::Safety::Unsafe => {
                                        suggestions.push((lo_span, format!("{attr}\n{indent}")));
                                    },
                                }

                                diag.multipart_suggestion(
                                    "add the missing target features to the surrounding function",
                                    suggestions,
                                    rustc_errors::Applicability::MaybeIncorrect,
                                );
                            },
                        );
                    }
                }
            }
            match &bb.terminator().kind {
                // Logically these add an outstanding block we never encounter
                // This means all remaining blocks are treated as conditional blocks
                TerminatorKind::Return
                | TerminatorKind::TailCall { .. }
                | TerminatorKind::Call { target: None, .. } => break,

                kind => {
                    for target in kind.successors() {
                        if !body.basic_blocks[target].is_cleanup {
                            outstanding_blocks += u32::from(seen_blocks.insert(target));
                        }
                    }
                },
            }
        }
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
