#![allow(clippy::similar_names)]
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;
use rustc_span::{sym, Symbol};

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
        if !in_external_macro(cx.tcx.sess, expr.span) {
            let Some(caller_def_id) = caller_def_id(cx, expr) else {
                return;
            };
            let Some(callee_def_id) = callee_def_id(cx, expr) else {
                return;
            };

            let caller_target_features = def_id_target_features(cx, caller_def_id);
            let callee_target_features = def_id_target_features(cx, callee_def_id);

            let missing: Vec<_> = callee_target_features
                .iter()
                .filter(|target_feature| !caller_target_features.contains(target_feature))
                .map(Symbol::as_str)
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

                    if let Some(caller_item) = caller_item(cx, expr) {
                        let hir::ItemKind::Fn(fn_sig, _, _) = caller_item.kind else {
                            unreachable!()
                        };

                        let mut suggestions = Vec::with_capacity(2);

                        let Some(indent) = indentation(cx, caller_item.span) else {
                            return;
                        };

                        let lo_span = caller_item.span.with_hi(caller_item.span.lo());

                        if let hir::Safety::Safe = fn_sig.header.safety {
                            if caller_item.vis_span.is_empty() {
                                suggestions.push((lo_span, format!("{attr}\n{indent}unsafe ")));
                            } else {
                                suggestions.push((lo_span, format!("{attr}\n{indent}")));
                                suggestions.push((caller_item.vis_span.shrink_to_hi(), " unsafe".to_string()));
                            }
                        }

                        diag.multipart_suggestion_verbose(
                            "add the missing target features to the surrounding function",
                            suggestions,
                            rustc_errors::Applicability::MaybeIncorrect,
                        );
                    }
                },
            );
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

fn caller_def_id<'tcx>(cx: &LateContext<'tcx>, expr: &hir::Expr<'tcx>) -> Option<DefId> {
    let item = caller_item(cx, expr)?;
    Some(item.owner_id.to_def_id())
}

fn caller_item<'tcx>(cx: &LateContext<'tcx>, expr: &hir::Expr<'tcx>) -> Option<&'tcx hir::Item<'tcx>> {
    for (_hir_id, node) in cx.tcx.hir().parent_iter(expr.hir_id) {
        if let hir::Node::Item(
            item @ hir::Item {
                kind: hir::ItemKind::Fn(..),
                ..
            },
        ) = node
        {
            return Some(item);
        }
    }

    None
}

// return the target features that the called function depends on
fn def_id_target_features(cx: &LateContext<'_>, did: DefId) -> Vec<Symbol> {
    let mut added_target_features = Vec::new();

    for attr in cx.tcx.get_attrs(did, sym::target_feature) {
        let Some(list) = attr.meta_item_list() else {
            return vec![];
        };

        for item in list {
            // Only `enable = ...` is accepted in the meta-item list.
            if !item.has_name(sym::enable) {
                continue;
            }

            // Must be of the form `enable = "..."` (a string).
            let Some(value) = item.value_str() else {
                continue;
            };

            added_target_features.extend(value.as_str().split(',').map(Symbol::intern));
        }
    }

    added_target_features
}

/// Returns the indentation before `span` if there are nothing but `[ \t]`
/// before it on its line.
fn indentation<T: rustc_lint::LintContext>(cx: &T, span: rustc_span::Span) -> Option<String> {
    let lo = cx.sess().source_map().lookup_char_pos(span.lo());
    lo.file
        .get_line(lo.line - 1 /* line numbers in `Loc` are 1-based */)
        .and_then(|line| {
            if let Some((pos, _)) = line.char_indices().find(|&(_, c)| c != ' ' && c != '\t') {
                // We can mix char and byte positions here because we only consider `[ \t]`.
                if lo.col == rustc_span::CharPos(pos) {
                    Some(line[..pos].into())
                } else {
                    None
                }
            } else {
                None
            }
        })
}
