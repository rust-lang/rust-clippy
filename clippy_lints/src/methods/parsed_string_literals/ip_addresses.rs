use std::net::{Ipv4Addr, Ipv6Addr};

use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::SpanRangeExt as _;
use clippy_utils::sym;
use rustc_hir::{Expr, QPath};
use rustc_lint::LateContext;
use rustc_span::Symbol;

use super::maybe_emit_lint;

static IPV4_ENTITY: &str = "an IPv4 address";
static IPV6_ENTITY: &str = "an IPv6 address";

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    lit: Symbol,
    method: Symbol,
    explicit_type: Option<QPath<'_>>,
    msrv: Msrv,
) {
    let ipaddr_consts_available = msrv.meets(cx, msrvs::IPADDR_CONSTANTS);
    match method {
        sym::Ipv4Addr => {
            // Only use constants such as `Ipv4Addr::LOCALHOST` when the type has been explicitly given
            if let Some((sugg, typed_const)) = ipv4_subst(cx, lit, ipaddr_consts_available, explicit_type) {
                maybe_emit_lint(cx, expr, typed_const, IPV4_ENTITY, sugg);
            }
        },
        sym::Ipv6Addr => {
            // Only use constants such as `Ipv4Addr::LOCALHOST` when the type has been explicitly given
            if let Some((sugg, typed_const)) = ipv6_subst(cx, lit, ipaddr_consts_available, explicit_type) {
                maybe_emit_lint(cx, expr, typed_const, IPV6_ENTITY, sugg);
            }
        },
        sym::IpAddr => {
            if let Some((sugg, entity)) = ip_subst(cx, lit, explicit_type) {
                maybe_emit_lint(cx, expr, false, entity, sugg);
            }
        },
        _ => unreachable!(),
    }
}

/// Suggests a replacement if `addr` is a correct IPv4 address, with:
/// - the replacement string
/// - a boolean that indicates if a typed constant is used
///
/// The replacement will be `T::CONSTANT` if a constant is detected,
/// where `T` is either `explicit_type` if provided, or `Ipv4Addr`
/// otherwise.
///
/// In other cases, when the type has been explicitly given as `T`, the
/// `T::new()` constructor will be used. If no type has been explicitly
/// given, then `[u8; 4].into()` will be used as the context should
/// already provide the proper information. This allows us not to use
/// a type name which might not be available in the current scope.
fn ipv4_subst(
    cx: &LateContext<'_>,
    addr: Symbol,
    with_consts: bool,
    explicit_type: Option<QPath<'_>>,
) -> Option<(String, bool)> {
    as_ipv4_octets(addr).and_then(|bytes| {
        if let Some(qpath) = explicit_type {
            qpath.span().with_source_text(cx, |ty| {
                if with_consts && &bytes == Ipv4Addr::LOCALHOST.as_octets() {
                    (format!("{ty}::LOCALHOST"), true)
                } else if with_consts && &bytes == Ipv4Addr::BROADCAST.as_octets() {
                    (format!("{ty}::BROADCAST"), true)
                } else if with_consts && &bytes == Ipv4Addr::UNSPECIFIED.as_octets() {
                    (format!("{ty}::UNSPECIFIED"), true)
                } else {
                    (
                        format!("{ty}::new({}, {}, {}, {})", bytes[0], bytes[1], bytes[2], bytes[3]),
                        false,
                    )
                }
            })
        } else {
            Some((
                format!("[{}, {}, {}, {}].into()", bytes[0], bytes[1], bytes[2], bytes[3]),
                false,
            ))
        }
    })
}

/// Try parsing `addr` as an IPv4 address and return its octets
fn as_ipv4_octets(addr: Symbol) -> Option<[u8; 4]> {
    addr.as_str().parse::<Ipv4Addr>().ok().map(|addr| *addr.as_octets())
}

/// Suggests a replacement if `addr` is a correct IPv6 address, with:
/// - the replacement string
/// - a boolean that indicates if a typed constant is used
///
/// Replacement will either be:
/// - `T::CONSTANT`
/// - `Ipv6Addr::CONSTANT` if no `explicit_type` is defined
/// - `T::new(…)`
/// - `[u16; 8].into()` if no `explicit_type` is defined
///
/// See [`ipv4_subst()`] for more details.
fn ipv6_subst(
    cx: &LateContext<'_>,
    addr: Symbol,
    with_consts: bool,
    explicit_type: Option<QPath<'_>>,
) -> Option<(String, bool)> {
    as_ipv6_segments(addr).and_then(|segments| {
        if let Some(qpath) = explicit_type {
            qpath.span().with_source_text(cx, |ty| {
                if with_consts && segments == Ipv6Addr::LOCALHOST.segments() {
                    (format!("{ty}::LOCALHOST"), true)
                } else if with_consts && explicit_type.is_some() && segments == Ipv6Addr::UNSPECIFIED.segments() {
                    (format!("{ty}::UNSPECIFIED"), true)
                } else {
                    (format!("{ty}::new({})", segments_to_string(&segments)), false)
                }
            })
        } else {
            Some((format!("[{}].into()", segments_to_string(&segments)), false))
        }
    })
}

/// Try parsing `addr` as an IPv6 address and return its 16-bit segments
fn as_ipv6_segments(addr: Symbol) -> Option<[u16; 8]> {
    addr.as_str().parse().ok().as_ref().map(Ipv6Addr::segments)
}

/// Return the `segments` separated by commas, in a common format for IPv6 addresses
fn segments_to_string(segments: &[u16; 8]) -> String {
    segments
        .map(|n| if n < 2 { n.to_string() } else { format!("{n:#x}") })
        .join(", ")
}

/// Suggests a replacement if `addr` is a correct IPv6 address, with:
/// - the replacement string
/// - the entity that was detected
///
/// `explicit_type` refers to `IpAddr`, and not to the content of one of the variants
/// (`IpAddr::V4` or `IpAddr::V6`). The use of constants from `Ipv4Addr` or `Ipv6Addr`
/// will not be proposed because we do not know if those types are imported in the scope.
fn ip_subst(cx: &LateContext<'_>, addr: Symbol, explicit_type: Option<QPath<'_>>) -> Option<(String, &'static str)> {
    if let Some([a0, a1, a2, a3]) = as_ipv4_octets(addr) {
        Some((
            if let Some(qpath) = explicit_type {
                qpath
                    .span()
                    .with_source_text(cx, |ty| format!("{ty}::V4([{a0}, {a1}, {a2}, {a3}].into())"))?
            } else {
                format!("[{a0}, {a1}, {a2}, {a3}].into()")
            },
            IPV4_ENTITY,
        ))
    } else if let Some(segments) = as_ipv6_segments(addr) {
        Some((
            if let Some(qpath) = explicit_type {
                qpath
                    .span()
                    .with_source_text(cx, |ty| format!("{ty}::V6([{}].into())", segments_to_string(&segments)))?
            } else {
                format!("[{}].into()", segments_to_string(&segments))
            },
            IPV6_ENTITY,
        ))
    } else {
        None
    }
}
