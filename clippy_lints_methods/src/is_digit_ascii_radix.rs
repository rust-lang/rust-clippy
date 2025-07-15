use clippy_utils::consts::{ConstEvalCtxt, FullInt};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Finds usages of [`char::is_digit`](https://doc.rust-lang.org/stable/std/primitive.char.html#method.is_digit) that
    /// can be replaced with [`is_ascii_digit`](https://doc.rust-lang.org/stable/std/primitive.char.html#method.is_ascii_digit) or
    /// [`is_ascii_hexdigit`](https://doc.rust-lang.org/stable/std/primitive.char.html#method.is_ascii_hexdigit).
    ///
    /// ### Why is this bad?
    /// `is_digit(..)` is slower and requires specifying the radix.
    ///
    /// ### Example
    /// ```no_run
    /// let c: char = '6';
    /// c.is_digit(10);
    /// c.is_digit(16);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let c: char = '6';
    /// c.is_ascii_digit();
    /// c.is_ascii_hexdigit();
    /// ```
    #[clippy::version = "1.62.0"]
    pub IS_DIGIT_ASCII_RADIX,
    style,
    "use of `char::is_digit(..)` with literal radix of 10 or 16"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    self_arg: &'tcx Expr<'_>,
    radix: &'tcx Expr<'_>,
    msrv: Msrv,
) {
    if !cx.typeck_results().expr_ty_adjusted(self_arg).peel_refs().is_char() {
        return;
    }

    if let Some(radix_val) = ConstEvalCtxt::new(cx).eval_full_int(radix) {
        let (num, replacement) = match radix_val {
            FullInt::S(10) | FullInt::U(10) => (10, "is_ascii_digit"),
            FullInt::S(16) | FullInt::U(16) => (16, "is_ascii_hexdigit"),
            _ => return,
        };
        let mut applicability = Applicability::MachineApplicable;

        if !msrv.meets(cx, msrvs::IS_ASCII_DIGIT) {
            return;
        }

        span_lint_and_sugg(
            cx,
            IS_DIGIT_ASCII_RADIX,
            expr.span,
            format!("use of `char::is_digit` with literal radix of {num}"),
            "try",
            format!(
                "{}.{replacement}()",
                snippet_with_applicability(cx, self_arg.span, "..", &mut applicability)
            ),
            applicability,
        );
    }
}
