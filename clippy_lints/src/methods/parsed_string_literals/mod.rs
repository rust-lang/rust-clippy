use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::Msrv;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::SpanRangeExt as _;
use clippy_utils::sym;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{self as hir, Expr, ExprKind, GenericArg, QPath};
use rustc_lint::LateContext;

mod ip_addresses;
mod primitive_types;

use super::PARSED_STRING_LITERALS;

/// Detects instances of `"literal".parse().unwrap()`:
/// - `expr` is the whole expression
/// - `recv` is the receiver of `parse()`
/// - `parse_call` is the `parse()` method call
/// - `msrv` is used for Rust version checking
pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, parse_call: &Expr<'_>, msrv: Msrv) {
    if let ExprKind::Lit(lit) = recv.kind
        && let LitKind::Str(lit, _) = lit.node
    {
        let ty = cx.typeck_results().expr_ty(expr);
        match ty.opt_diag_name(cx) {
            _ if ty.is_primitive() => primitive_types::check(cx, expr, lit, ty, recv, type_from_parse(parse_call)),
            Some(method @ (sym::IpAddr | sym::Ipv4Addr | sym::Ipv6Addr)) => {
                ip_addresses::check(cx, expr, lit, method, type_from_parse(parse_call), msrv);
            },
            _ => (),
        }
    }
}

/// Emit the lint if the length of `sugg` is no longer than the original `expr` span, or if `force`
/// is set.
fn maybe_emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, force: bool, entity: &str, sugg: String) {
    if force || expr.span.check_source_text(cx, |snip| snip.len() >= sugg.len()) {
        span_lint_and_sugg(
            cx,
            PARSED_STRING_LITERALS,
            expr.span,
            format!("unnecessary runtime parsing of {entity}"),
            "use",
            sugg,
            Applicability::MachineApplicable,
        );
    }
}

/// Returns `T` from the `parse::<T>(…)` call if present.
fn type_from_parse<'hir>(parse_call: &'hir Expr<'_>) -> Option<QPath<'hir>> {
    if let ExprKind::MethodCall(parse, _, _, _) = parse_call.kind
        && let [GenericArg::Type(ty)] = parse.args().args
        && let hir::TyKind::Path(qpath) = ty.kind
    {
        Some(qpath)
    } else {
        None
    }
}
