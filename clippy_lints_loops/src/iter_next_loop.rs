use clippy_utils::diagnostics::span_lint;
use clippy_utils::is_trait_method;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for loops on `x.next()`.
    ///
    /// ### Why is this bad?
    /// `next()` returns either `Some(value)` if there was a
    /// value, or `None` otherwise. The insidious thing is that `Option<_>`
    /// implements `IntoIterator`, so that possibly one value will be iterated,
    /// leading to some hard to find bugs. No one will want to write such code
    /// [except to win an Underhanded Rust
    /// Contest](https://www.reddit.com/r/rust/comments/3hb0wm/underhanded_rust_contest/cu5yuhr).
    ///
    /// ### Example
    /// ```ignore
    /// for x in y.next() {
    ///     ..
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ITER_NEXT_LOOP,
    correctness,
    "for-looping over `_.next()` which is probably not intended"
}

pub(super) fn check(cx: &LateContext<'_>, arg: &Expr<'_>) {
    if is_trait_method(cx, arg, sym::Iterator) {
        span_lint(
            cx,
            ITER_NEXT_LOOP,
            arg.span,
            "you are iterating over `Iterator::next()` which is an Option; this will compile but is \
            probably not what you want",
        );
    }
}
