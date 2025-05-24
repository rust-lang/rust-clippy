use std::borrow::Cow;
use std::net::{Ipv4Addr, Ipv6Addr};

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::SpanRangeExt;
use clippy_utils::sym;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;

use super::PARSED_STRING_LITERALS;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, msrv: Msrv) {
    if let ExprKind::Lit(lit) = recv.kind
        && let LitKind::Str(str, _) = lit.node
    {
        let consts_available = || msrv.meets(cx, msrvs::IPADDR_CONSTANTS);
        let ty = cx.typeck_results().expr_ty(expr);
        if is_type_diagnostic_item(cx, ty, sym::Ipv4Addr)
            && let Some(sugg) = ipv4_subst(str, consts_available())
        {
            maybe_emit_lint(cx, expr, sugg.is_borrowed(), sugg);
        } else if is_type_diagnostic_item(cx, ty, sym::Ipv6Addr)
            && let Some(sugg) = ipv6_subst(str, consts_available())
        {
            maybe_emit_lint(cx, expr, sugg.is_borrowed(), sugg);
        } else if is_type_diagnostic_item(cx, ty, sym::IpAddr) {
            let with_consts = consts_available();
            if let Some(sugg) = ipv4_subst(str, with_consts) {
                maybe_emit_lint(cx, expr, sugg.is_borrowed(), format!("IpAddr::V4({sugg})").into());
            } else if let Some(sugg) = ipv6_subst(str, with_consts) {
                maybe_emit_lint(cx, expr, sugg.is_borrowed(), format!("IpAddr::V6({sugg})").into());
            }
        }
    }
}

/// Suggests a replacement if `addr` is a correct IPv4 address
fn ipv4_subst(addr: Symbol, with_consts: bool) -> Option<Cow<'static, str>> {
    addr.as_str().parse().ok().map(|ipv4: Ipv4Addr| {
        if with_consts && ipv4.as_octets() == &[127, 0, 0, 1] {
            "Ipv4Addr::LOCALHOST".into()
        } else if with_consts && ipv4.is_broadcast() {
            "Ipv4Addr::BROADCAST".into()
        } else if with_consts && ipv4.is_unspecified() {
            "Ipv4Addr::UNSPECIFIED".into()
        } else {
            let ipv4 = ipv4.as_octets();
            format!("Ipv4Addr::new({}, {}, {}, {})", ipv4[0], ipv4[1], ipv4[2], ipv4[3]).into()
        }
    })
}

/// Suggests a replacement if `addr` is a correct IPv6 address
fn ipv6_subst(addr: Symbol, with_consts: bool) -> Option<Cow<'static, str>> {
    addr.as_str().parse().ok().map(|ipv6: Ipv6Addr| {
        if with_consts && ipv6.is_loopback() {
            "Ipv6Addr::LOCALHOST".into()
        } else if with_consts && ipv6.is_unspecified() {
            "Ipv6Addr::UNSPECIFIED".into()
        } else {
            format!(
                "Ipv6Addr::new([{}])",
                ipv6.segments()
                    .map(|n| if n < 2 { n.to_string() } else { format!("{n:#x}") })
                    .join(", ")
            )
            .into()
        }
    })
}

/// Emit the lint if the length of `sugg` is no longer than the original `expr` span, or if `force`
/// is set.
fn maybe_emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, force: bool, sugg: Cow<'_, str>) {
    if force || expr.span.check_source_text(cx, |snip| snip.len() >= sugg.len()) {
        span_lint_and_sugg(
            cx,
            PARSED_STRING_LITERALS,
            expr.span,
            "unnecessary runtime parsing of IP address",
            "use",
            sugg.into(),
            Applicability::MaybeIncorrect,
        );
    }
}
