use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::DiagExt;
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
    #[clippy::version = "CURRENT_CLIPPY_VERSION"]
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
                .map(|target_feature| target_feature.as_str())
                .collect();

            if missing.is_empty() {
                return;
            }

            let hint = format!("#[target_feature(enable = \"{}\")]", missing.join(","));

            #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
            span_lint_and_then(
                cx,
                CALL_MISSING_TARGET_FEATURE,
                expr.span,
                "this call requires target features that the surrounding function does not enable",
                |diag| {
                    diag.span_label(
                        expr.span,
                        format!("this function call requires target features to be enabled"),
                    );

                    if let Some(caller_item) = caller_item(cx, expr) {
                        diag.suggest_item_with_attr(
                            cx,
                            caller_item.span,
                            "add the missing target features to the surrounding function",
                            hint.as_str(),
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

            added_target_features.extend(value.as_str().split(',').map(|feature| Symbol::intern(feature)));
        }
    }

    added_target_features
}
