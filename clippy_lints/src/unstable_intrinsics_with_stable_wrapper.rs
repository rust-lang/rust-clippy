use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind, Item, ItemKind, PathSegment, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_resolve::rustdoc::{add_doc_fragment, attrs_to_doc_fragments};
use rustc_session::declare_lint_pass;
use rustc_span::def_id::DefId;
use rustc_span::{sym, Span};

use clippy_utils::diagnostics::span_lint;
use clippy_utils::{any_parent_has_attr, def_path_def_ids, match_any_def_paths};

use std::sync::OnceLock;

declare_clippy_lint! {
    /// ### What it does
    /// Detects usage of unstable intrinsics with safe wrappers.
    ///
    /// ### Why is this bad?
    /// It allows to have the same features without requiring to use the `core_intrinsics`
    /// feature.
    ///
    /// ### Example
    /// ```no_run
    /// # #![feature(core_intrinsics)]
    /// use std::intrinsics::add_with_overflow;
    ///
    /// add_with_overflow(12u32, 14);
    /// ```
    /// Use instead:
    /// ```no_run
    /// 12u32.overflowing_add(14);
    /// ```
    #[clippy::version = "1.77.0"]
    pub UNSTABLE_INTRINSICS_WITH_STABLE_WRAPPER,
    suspicious,
    "Detects usage of unstable intrinsics with safe wrappers"
}

declare_lint_pass!(UnstableIntrinsicsWithStableWrapper => [UNSTABLE_INTRINSICS_WITH_STABLE_WRAPPER]);

fn get_doc(tcx: TyCtxt<'_>, fn_def_id: DefId) -> String {
    let (fragments, _) =
        attrs_to_doc_fragments(tcx.get_attrs_unchecked(fn_def_id).iter().map(|attr| (attr, None)), true);
    let mut doc = String::new();
    for fragment in &fragments {
        add_doc_fragment(&mut doc, fragment);
    }
    doc
}

fn emit_if_is_unstable_intrinsic(
    cx: &LateContext<'_>,
    def_id: DefId,
    expr_span: Span,
    fn_name: &str,
    segments: &[PathSegment<'_>],
) {
    static FNS: OnceLock<Vec<DefId>> = OnceLock::new();
    let fns = FNS.get_or_init(|| {
        let mod_def_id = def_path_def_ids(cx, &["core", "intrinsics"]).next().unwrap();
        cx.tcx
            .module_children(mod_def_id)
            .iter()
            .filter_map(|child| match child.res {
                Res::Def(DefKind::Fn, fn_def_id) => Some(fn_def_id),
                _ => None,
            })
            .filter(|fn_def_id| !get_doc(cx.tcx, *fn_def_id).contains("does not have a stable counterpart"))
            .collect::<Vec<_>>()
    });

    if cx.tcx.is_intrinsic(def_id)
        // This is to prevent false positives like "transmute".
        && segments.len() > 1
        && let Some(mod_def_id) = segments[segments.len() - 2].res.opt_def_id()
        && match_any_def_paths(cx, mod_def_id, &[
            &["core", "intrinsics"],
            &["std", "intrinsics"],
        ]).is_some()
    {
        for intrinsic in fns {
            if *intrinsic == def_id {
                span_lint(
                    cx,
                    UNSTABLE_INTRINSICS_WITH_STABLE_WRAPPER,
                    expr_span,
                    &format!(
                        "consider using the stable counterpart mentioned in the documentation \
                         (https://doc.rust-lang.org/stable/core/intrinsics/fn.{fn_name}.html)"
                    ),
                );
                return;
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnstableIntrinsicsWithStableWrapper {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Use(path, _) = item.kind {
            for res in &path.res {
                if let Some(use_def_id) = res.opt_def_id()
                    && cx.tcx.def_kind(use_def_id) == DefKind::Fn
                    && let Some(fn_name) = cx.tcx.opt_item_name(use_def_id)
                {
                    emit_if_is_unstable_intrinsic(cx, use_def_id, path.span, fn_name.as_str(), path.segments);
                }
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(call_expr, _) = expr.kind
            && let ExprKind::Path(QPath::Resolved(_, path)) = call_expr.kind
            && let Some(fn_def_id) = path.res.opt_def_id()
            && !fn_def_id.is_local()
            // We get the function name instead from `path` because it could have been renamed.
            && let Some(fn_name) = cx.tcx.opt_item_name(fn_def_id)
            && let fn_name = fn_name.as_str()
            && !any_parent_has_attr(cx.tcx, expr.hir_id, sym::automatically_derived)
        {
            emit_if_is_unstable_intrinsic(cx, fn_def_id, expr.span, fn_name, path.segments);
        }
    }
}
