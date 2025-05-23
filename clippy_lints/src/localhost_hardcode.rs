use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_integer_const;
use clippy_utils::source::SpanRangeExt;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects hardcoded localhost IP addresses using `Ipv4Addr::new(127, 0, 0, 1)`.
    ///
    /// ### Why is this bad?
    /// Using a hardcoded IP address `(127.0.0.1)` is less clear and maintainable than using the
    /// `Ipv4Addr::LOCALHOST` constant.
    ///
    /// ### Example
    /// ```no_run
    /// use std::net::Ipv4Addr;
    /// let addr = Ipv4Addr::new(127, 0, 0, 1);
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::net::Ipv4Addr;
    /// let addr = Ipv4Addr::LOCALHOST;
    /// ```
    #[clippy::version = "1.89.0"]
    pub LOCALHOST_HARDCODE,
    style,
    "hardcoded localhost IP address"
}
declare_lint_pass!(LocalhostHardcode => [LOCALHOST_HARDCODE]);

fn is_localhost_hardcode(cx: &LateContext<'_>, args: &'_ [Expr<'_>]) -> bool {
    if args.len() == 4
        && args.iter().enumerate().all(|(i, arg)| match i {
            0 => is_integer_const(cx, arg, 127),
            1 | 2 => is_integer_const(cx, arg, 0),
            3 => is_integer_const(cx, arg, 1),
            _ => false,
        })
    {
        return true;
    }
    false
}

impl LateLintPass<'_> for LocalhostHardcode {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        // Ignore expressions from macro expansions
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Call(func, args) = &expr.kind
            && let ExprKind::Path(qpath) = &func.kind
            && let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id()
            && "std::net::Ipv4Addr::new" == cx.tcx.def_path_str(def_id)
            && is_localhost_hardcode(cx, args)
            // Get source text of the calling relative path for suggestion
            && let QPath::TypeRelative(ty, _) = qpath
            && let Some(ty_span) = ty.span.get_source_range(cx)
            && let Some(ty_text) = ty_span.as_str()
        {
            let suggestion = format!("{ty_text}::LOCALHOST");

            span_lint_and_sugg(
                cx,
                LOCALHOST_HARDCODE,
                expr.span,
                format!("use `{suggestion}` instead"),
                "try",
                suggestion,
                Applicability::MachineApplicable,
            );
        }
    }
}
