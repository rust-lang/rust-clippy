use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_span::sym;
use smallvec::SmallVec;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for IP addresses that could be replaced with predefined constants such as
    /// `Ipv4Addr::new(127, 0, 0, 1)` instead of using the appropriate constants.
    ///
    /// ### Why is this bad?
    /// Using specific IP addresses like `127.0.0.1` or `::1` is less clear and less maintainable than using the
    /// predefined constants `Ipv4Addr::LOCALHOST` or `Ipv6Addr::LOCALHOST`. These constants improve code
    /// readability, make the intent explicit, and are less error-prone.
    ///
    /// ### Example
    /// ```no_run
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// // IPv4 loopback
    /// let addr_v4 = Ipv4Addr::new(127, 0, 0, 1);
    ///
    /// // IPv6 loopback
    /// let addr_v6 = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::net::{Ipv4Addr, Ipv6Addr};
    ///
    /// // IPv4 loopback
    /// let addr_v4 = Ipv4Addr::LOCALHOST;
    ///
    /// // IPv6 loopback
    /// let addr_v6 = Ipv6Addr::LOCALHOST;
    /// ```
    #[clippy::version = "1.89.0"]
    pub IP_CONSTANT,
    pedantic,
    "hardcoded localhost IP address"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, func: &Expr<'_>, args: &[Expr<'_>]) {
    if let ExprKind::Path(QPath::TypeRelative(
        Ty {
            kind: TyKind::Path(QPath::Resolved(_, func_path)),
            ..
        },
        p,
    )) = func.kind
        && p.ident.name == sym::new
        && let Some(func_def_id) = func_path.res.opt_def_id()
        && matches!(
            cx.tcx.get_diagnostic_name(func_def_id),
            Some(sym::Ipv4Addr | sym::Ipv6Addr)
        )
        && let Some(args) = args
            .iter()
            .map(|arg| {
                if let Some(Constant::Int(constant @ (0 | 1 | 127 | 255))) = ConstEvalCtxt::new(cx).eval(arg) {
                    u8::try_from(constant).ok()
                } else {
                    None
                }
            })
            .collect::<Option<SmallVec<[u8; 8]>>>()
    {
        let constant_name = match args.as_slice() {
            [0, 0, 0, 0] | [0, 0, 0, 0, 0, 0, 0, 0] => "UNSPECIFIED",
            [127, 0, 0, 1] | [0, 0, 0, 0, 0, 0, 0, 1] => "LOCALHOST",
            [255, 255, 255, 255] => "BROADCAST",
            _ => return,
        };

        span_lint_and_then(cx, IP_CONSTANT, expr.span, "hand-coded well-known IP address", |diag| {
            diag.span_suggestion_verbose(
                expr.span.with_lo(p.ident.span.lo()),
                "use",
                constant_name,
                Applicability::MachineApplicable,
            );
        });
    }
}
