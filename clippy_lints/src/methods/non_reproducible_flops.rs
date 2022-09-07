use clippy_utils::diagnostics::span_lint;
use clippy_utils::path_def_id;
use rustc_hir::{self as hir, Expr};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::Ident;

use super::NON_REPRODUCIBLE_FLOPS;

#[rustfmt::skip]
static WHITELIST_PREFIX: &[&str] = &[
    "is_", "from_", "to_",
];

#[rustfmt::skip]
static WHITELIST: &[&str] = &[
    "clone", "clone_from", "fmt", "default",
    "from", "into", "try_from", "try_into", "from_str",

    "floor", "ceil", "round", "trunc", "fract",
    "next_up", "next_down",

    "add", "sub", "mul", "div", "rem", "mul_add",
    "add_assign", "sub_assign", "mul_assign", "div_assign", "rem_assign",
    "min", "max", "minimum", "maximum", "clamp",

    "neg", "abs", "copysign",

    "eq", "ne", "cmp", "partial_eq", "partial_cmp",
    "lt", "le", "gt", "ge", "total_cmp",

    "classify",
];

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
    let ident = match expr.kind {
        hir::ExprKind::Call(func, _) => {
            let def_id = if let Some(def_id) = path_def_id(cx, func) {
                def_id
            } else {
                return;
            };

            let path = cx.get_def_path(def_id);

            let crates = &[rustc_span::sym::core, rustc_span::sym::std];
            let types = &[rustc_span::sym::f32, rustc_span::sym::f64];

            let is_float_fn = crates
                .iter()
                .copied()
                .flat_map(|c| types.iter().copied().map(move |t| [c, t]))
                .any(|syms| syms.iter().copied().zip(path.iter().copied()).all(|(a, b)| a == b));

            if is_float_fn {
                if let Some(sym) = path.last() {
                    Ident::new(*sym, func.span)
                } else {
                    return;
                }
            } else {
                return;
            }
        },
        hir::ExprKind::MethodCall(path, [recv, args @ ..], _) => {
            let ty = cx.typeck_results().expr_ty(recv);
            if matches!(ty.kind(), ty::Float(_)) && !args.iter().any(|e| e.span.from_expansion()) {
                path.ident
            } else {
                return;
            }
        },
        _ => return,
    };

    let name_str = ident.name.as_str();

    if WHITELIST_PREFIX.iter().any(|p| name_str.starts_with(p)) {
        return;
    }

    if WHITELIST.contains(&name_str) {
        return;
    }

    span_lint(
        cx,
        NON_REPRODUCIBLE_FLOPS,
        ident.span,
        "non-reproducible float operation detected",
    );
}
