use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::IPV4V6_CONSTANT_HARDCODE;

static IPV4V6_CONSTANTS: &[(&[u128], &str)] = &[
    // Ipv4
    (&[127, 0, 0, 1], "LOCALHOST"),
    (&[255, 255, 255, 255], "BROADCAST"),
    (&[0, 0, 0, 0], "UNSPECIFIED"),
    // Ipv6
    (&[0, 0, 0, 0, 0, 0, 0, 1], "LOCALHOST"),
    (&[0, 0, 0, 0, 0, 0, 0, 0], "UNSPECIFIED"),
];

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, func: &Expr<'_>, args: &[Expr<'_>]) {
    if let ExprKind::Path(QPath::TypeRelative(
        Ty {
            kind: TyKind::Path(QPath::Resolved(_, func_path)),
            ..
        },
        p,
    )) = func.kind
        && p.ident.as_str() == "new"
        && let Some(func_def_id) = func_path.res.opt_def_id()
        && (cx.tcx.is_diagnostic_item(sym::Ipv4Addr, func_def_id)
            || cx.tcx.is_diagnostic_item(sym::Ipv6Addr, func_def_id))
        && let Some(constant_name) = is_hardcoded_ipv4v6_constant(cx, args)
    {
        let sugg_snip = format!(
            "{}::{}",
            snippet(cx, func_path.span, cx.tcx.def_path_str(func_def_id).as_str()),
            constant_name
        );

        span_lint_and_sugg(
            cx,
            IPV4V6_CONSTANT_HARDCODE,
            expr.span,
            format!("use `{sugg_snip}` instead"),
            "try",
            sugg_snip,
            Applicability::MachineApplicable,
        );
    }
}

fn is_hardcoded_ipv4v6_constant(cx: &LateContext<'_>, args: &[Expr<'_>]) -> Option<&'static str> {
    if args.len() != 4 && args.len() != 8 {
        return None;
    }

    // Extract integer constants from arguments
    let mut constants = Vec::new();
    for arg in args {
        if let Some(Constant::Int(constant)) = ConstEvalCtxt::new(cx).eval(arg) {
            constants.push(constant);
        } else {
            return None;
        }
    }
    // Check against known IP constants
    for (pattern, name) in IPV4V6_CONSTANTS {
        if pattern.len() == constants.len() && pattern.iter().eq(constants.iter()) {
            return Some(name);
        }
    }

    None
}
