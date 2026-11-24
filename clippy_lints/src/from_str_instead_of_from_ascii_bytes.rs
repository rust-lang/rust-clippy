use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet;
use clippy_utils::sym;
use rustc_hir::{Expr, ExprKind, QPath, Ty};
use rustc_lint::{Applicability, LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty;
use rustc_span::symbol::Symbol;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects calls to integer `from_ascii_bytes` methods when the input is a string slice,
    /// and suggests using the corresponding string parsing method instead.
    ///
    /// ### Why is this bad?
    ///
    /// `from_ascii_bytes` is intended for parsing byte slices. When the input is already a
    /// string slice, using `parse` or `from_str_radix` expresses that intent more directly.
    ///
    /// ### Example
    /// ```no_run
    /// let decimal = u32::from_ascii_bytes("123");
    /// let hex = u32::from_ascii_bytes_radix("ff", 16);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let decimal = "123".parse::<u32>();
    /// let hex = u32::from_str_radix("ff", 16);
    /// ```
    #[clippy::version = "1.102.0"]
    pub FROM_STR_INSTEAD_OF_FROM_ASCII_BYTES,
    style,
    "using `from_ascii_bytes` with a string slice"
}

declare_lint_pass!(FromStrInsteadOfFromAsciiBytes => [
    FROM_STR_INSTEAD_OF_FROM_ASCII_BYTES,
]);

impl<'tcx> LateLintPass<'tcx> for FromStrInsteadOfFromAsciiBytes {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(func, args) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, path_segment)) = func.kind
            && is_integer_or_nonzero_integer(cx, ty)
            && let Some(true) = args.first().map(|arg| is_str(cx, arg))
            && let Some((replacement, help)) = suggestion(cx, ty, path_segment.ident.name, args)
        {
            span_lint_and_sugg(
                cx,
                FROM_STR_INSTEAD_OF_FROM_ASCII_BYTES,
                expr.span,
                format!("`{}` is used with str", path_segment.ident.name),
                help,
                replacement,
                Applicability::MachineApplicable,
            );
        }
    }
}

fn suggestion(
    cx: &LateContext<'_>,
    ty: &'_ Ty<'_>,
    path_segment_name: Symbol,
    args: &'_ [Expr<'_>],
) -> Option<(String, &'static str)> {
    match path_segment_name {
        sym::from_ascii_bytes_radix => {
            let ty_snippet = if let ty::Adt(..) = cx.typeck_results().node_type(ty.hir_id).kind() {
                format!("<{}>", snippet(cx, ty.span, ".."))
            } else {
                format!("{}", snippet(cx, ty.span, ".."))
            };

            Some((
                format!(
                    "{}::from_str_radix({}, {})",
                    ty_snippet,
                    snippet(cx, args.first()?.span, ".."),
                    snippet(cx, args.get(1)?.span, "..")
                ),
                "use `from_str_radix` instead",
            ))
        },
        sym::from_ascii_bytes => Some((
            format!(
                "{}.parse::<{}>()",
                snippet(cx, args.first()?.span, ".."),
                snippet(cx, ty.span, "..")
            ),
            "use `parse` method instead",
        )),
        _ => None,
    }
}

fn is_integer_or_nonzero_integer(cx: &LateContext<'_>, ty: &Ty<'_>) -> bool {
    let ty = cx.typeck_results().node_type(ty.hir_id);

    ty.is_integral() || ty.is_diag_item(cx, sym::NonZero)
}

fn is_str(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ty::Ref(_, inner_ty, _) = cx.typeck_results().expr_ty(expr).kind() {
        return inner_ty.is_str();
    }

    false
}
