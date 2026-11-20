use rustc_ast::ast::*;
use rustc_lint::{EarlyContext, EarlyLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for floating-point sorting comparators that use `partial_cmp`
    /// with `Ordering::Equal` as a fallback for incomparable values.
    ///
    /// ### Why is this bad?
    /// `partial_cmp` returns `None` when either operand is NaN. Replacing
    /// `None` with `Ordering::Equal` treats NaN as equal to every other value.
    ///
    /// This violates the transitivity required by sorting algorithms. For
    /// example, If 1.0 and NaN compare equal, and NaN and 2.0 compare equal,
    /// then 1.0 and 2.0 must also compare equal. But they don't.
    ///
    /// As a result, sorting may panic or produce incorrectly ordered results.
    ///
    /// ### Example
    /// ```no_run
    /// use std::cmp::Ordering;
    ///
    /// let mut values = vec![1.0_f64, f64::NAN, 2.0];
    /// values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut values = vec![1.0_f64, f64::NAN, 2.0];
    /// values.sort_by(f64::total_cmp);
    /// ```
    ///
    /// Note that `total_cmp` has different semantics: it distinguishes positive
    /// and negative zero and assigns NaNs a defined ordering.
    ///
    /// If the collection is guaranteed not to contain NaNs, use expect instead
    /// to make that assumption explicit:
    /// ```no_run
    /// values.sort_by(|a, b| a.partial_cmp(b).expect("values must not contain NaNs"));
    /// ```
    ///
    /// This preserves the usual floating-point comparison semantics and panics
    /// if the assumption is violated, rather than silently producing an invalid
    /// ordering.
    #[clippy::version = "1.101.0"]
    pub INVALID_FLOAT_SORT_COMPARATOR,
    suspicious,
    "floating-point sorting comparator may violate total ordering"
}
declare_lint_pass!(InvalidFloatSortComparator => [INVALID_FLOAT_SORT_COMPARATOR]);

impl EarlyLintPass for InvalidFloatSortComparator {}
