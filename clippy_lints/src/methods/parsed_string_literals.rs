use std::borrow::Cow;
use std::fmt::Display;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{SpanRangeExt, str_literal_to_char_literal};
use clippy_utils::sym;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Symbol;

use super::PARSED_STRING_LITERALS;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, msrv: Msrv) {
    if let ExprKind::Lit(lit) = recv.kind
        && let LitKind::Str(lit, _) = lit.node
    {
        let ty = cx.typeck_results().expr_ty(expr);
        if !check_primitive(cx, expr, lit, ty, recv) {
            check_ipaddr(cx, expr, lit, ty, msrv);
        }
    }
}

fn check_primitive(cx: &LateContext<'_>, expr: &Expr<'_>, lit: Symbol, ty: Ty<'_>, strlit: &Expr<'_>) -> bool {
    macro_rules! number {
        ($kind:ident, $expr:expr, $msg:expr, [$($subkind:ident => $ty:ident),*$(,)?]$(,)?) => {{
            match $expr {
                $(ty::$kind::$subkind => (try_parse::<$ty>(lit, Some(stringify!($ty))), $msg),)*
                #[allow(unreachable_patterns)]
                _ => return false,
            }
        }};
    }

    let mut app = Applicability::MachineApplicable;
    if let (Some(subst), entity) = match ty.kind() {
        ty::Int(int_ty) => number!(IntTy, int_ty, "a signed integer",
                                     [Isize => isize, I8 => i8, I16 => i16, I32 => i32, I64 => i64, I128 => i128]),
        ty::Uint(uint_ty) => number!(UintTy, uint_ty, "an unsigned integer",
                                        [Usize => usize, U8 => u8, U16 => u16, U32 => u32, U64 => u64, U128 => u128]),
        // FIXME: ignore `f16` and `f128` for now as they cannot use the default formatter
        ty::Float(float_ty) => number!(FloatTy, float_ty, "a real number",
                                           [F32 => f32, F64 => f64]),
        ty::Bool => (try_parse::<bool>(lit, None), "a boolean"),
        ty::Char => (str_literal_to_char_literal(cx, strlit, &mut app, false), "a character"),
        _ => return false,
    } {
        maybe_emit_lint(cx, expr, false, entity, subst.into(), app);
    }
    true
}

fn check_ipaddr(cx: &LateContext<'_>, expr: &Expr<'_>, lit: Symbol, ty: Ty<'_>, msrv: Msrv) {
    static IPV4_ENTITY: &str = "an IPv4 address";
    static IPV6_ENTITY: &str = "an IPv6 address";
    let ipaddr_consts_available = || msrv.meets(cx, msrvs::IPADDR_CONSTANTS);
    if is_type_diagnostic_item(cx, ty, sym::Ipv4Addr)
        && let Some(sugg) = ipv4_subst(lit, ipaddr_consts_available())
    {
        maybe_emit_lint(
            cx,
            expr,
            sugg.is_borrowed(),
            IPV4_ENTITY,
            sugg,
            Applicability::MaybeIncorrect,
        );
    } else if is_type_diagnostic_item(cx, ty, sym::Ipv6Addr)
        && let Some(sugg) = ipv6_subst(lit, ipaddr_consts_available())
    {
        maybe_emit_lint(
            cx,
            expr,
            sugg.is_borrowed(),
            IPV6_ENTITY,
            sugg,
            Applicability::MaybeIncorrect,
        );
    } else if is_type_diagnostic_item(cx, ty, sym::IpAddr) {
        let with_consts = ipaddr_consts_available();
        if let Some(sugg) = ipv4_subst(lit, with_consts) {
            maybe_emit_lint(
                cx,
                expr,
                sugg.is_borrowed(),
                IPV4_ENTITY,
                format!("IpAddr::V4({sugg})").into(),
                Applicability::MaybeIncorrect,
            );
        } else if let Some(sugg) = ipv6_subst(lit, with_consts) {
            maybe_emit_lint(
                cx,
                expr,
                sugg.is_borrowed(),
                IPV6_ENTITY,
                format!("IpAddr::V6({sugg})").into(),
                Applicability::MaybeIncorrect,
            );
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

fn try_parse<T: FromStr + Display>(lit: Symbol, suffix: Option<&str>) -> Option<String> {
    lit.as_str()
        .parse::<T>()
        .ok()
        .map(|_| suffix.map_or_else(|| lit.to_string(), |suffix| format!("{lit}_{suffix}")))
}

/// Emit the lint if the length of `sugg` is no longer than the original `expr` span, or if `force`
/// is set.
fn maybe_emit_lint(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    force: bool,
    entity: &str,
    sugg: Cow<'_, str>,
    applicability: Applicability,
) {
    if force || expr.span.check_source_text(cx, |snip| snip.len() >= sugg.len()) {
        span_lint_and_sugg(
            cx,
            PARSED_STRING_LITERALS,
            expr.span,
            format!("unnecessary runtime parsing of {entity}"),
            "use",
            sugg.into(),
            applicability,
        );
    }
}
