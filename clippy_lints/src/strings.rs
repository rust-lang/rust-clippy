use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_type_lang_item;
use clippy_utils::{
    SpanlessEq, get_parent_expr, is_lint_allowed, peel_blocks,
};
use rustc_hir::{BinOpKind, Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::source_map::Spanned;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for string appends of the form `x = x + y` (without
    /// `let`!).
    ///
    /// ### Why is this bad?
    /// It's not really bad, but some people think that the
    /// `.push_str(_)` method is more readable.
    ///
    /// ### Example
    /// ```no_run
    /// let mut x = "Hello".to_owned();
    /// x = x + ", World";
    ///
    /// // More readable
    /// x += ", World";
    /// x.push_str(", World");
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub STRING_ADD_ASSIGN,
    pedantic,
    "using `x = x + ..` where x is a `String` instead of `push_str()`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for all instances of `x + _` where `x` is of type
    /// `String`, but only if [`string_add_assign`](#string_add_assign) does *not*
    /// match.
    ///
    /// ### Why restrict this?
    /// This particular
    /// `Add` implementation is asymmetric (the other operand need not be `String`,
    /// but `x` does), while addition as mathematically defined is symmetric, and
    /// the `String::push_str(_)` function is a perfectly good replacement.
    /// Therefore, some dislike it and wish not to have it in their code.
    ///
    /// That said, other people think that string addition, having a long tradition
    /// in other languages is actually fine, which is why we decided to make this
    /// particular lint `allow` by default.
    ///
    /// ### Example
    /// ```no_run
    /// let x = "Hello".to_owned();
    /// x + ", World";
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut x = "Hello".to_owned();
    /// x.push_str(", World");
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub STRING_ADD,
    restriction,
    "using `x + ..` where x is a `String` instead of `push_str()`"
}




declare_clippy_lint! {
    /// ### What it does
    /// Checks for slice operations on strings
    ///
    /// ### Why restrict this?
    /// UTF-8 characters span multiple bytes, and it is easy to inadvertently confuse character
    /// counts and string indices. This may lead to panics, and should warrant some test cases
    /// containing wide UTF-8 characters. This lint is most useful in code that should avoid
    /// panics at all costs.
    ///
    /// ### Known problems
    /// Probably lots of false positives. If an index comes from a known valid position (e.g.
    /// obtained via `char_indices` over the same string), it is totally OK.
    ///
    /// ### Example
    /// ```rust,should_panic
    /// &"Ölkanne"[1..];
    /// ```
    #[clippy::version = "1.58.0"]
    pub STRING_SLICE,
    restriction,
    "slicing a string"
}

declare_lint_pass!(StringAdd => [STRING_ADD, STRING_ADD_ASSIGN, STRING_SLICE]);

impl<'tcx> LateLintPass<'tcx> for StringAdd {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if e.span.in_external_macro(cx.sess().source_map()) {
            return;
        }
        match e.kind {
            ExprKind::Binary(
                Spanned {
                    node: BinOpKind::Add, ..
                },
                left,
                _,
            ) => {
                if is_string(cx, left) {
                    if !is_lint_allowed(cx, STRING_ADD_ASSIGN, e.hir_id) {
                        let parent = get_parent_expr(cx, e);
                        if let Some(p) = parent {
                            if let ExprKind::Assign(target, _, _) = p.kind {
                                // avoid duplicate matches
                                if SpanlessEq::new(cx).eq_expr(target, left) {
                                    return;
                                }
                            }
                        }
                    }
                    span_lint(
                        cx,
                        STRING_ADD,
                        e.span,
                        "you added something to a string. Consider using `String::push_str()` instead",
                    );
                }
            },
            ExprKind::Assign(target, src, _) => {
                if is_string(cx, target) && is_add(cx, src, target) {
                    span_lint(
                        cx,
                        STRING_ADD_ASSIGN,
                        e.span,
                        "you assigned the result of adding something to this string. Consider using \
                         `String::push_str()` instead",
                    );
                }
            },
            ExprKind::Index(target, _idx, _) => {
                let e_ty = cx.typeck_results().expr_ty_adjusted(target).peel_refs();
                if e_ty.is_str() || is_type_lang_item(cx, e_ty, LangItem::String) {
                    span_lint(
                        cx,
                        STRING_SLICE,
                        e.span,
                        "indexing into a string may panic if the index is within a UTF-8 character",
                    );
                }
            },
            _ => {},
        }
    }
}

fn is_string(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    is_type_lang_item(cx, cx.typeck_results().expr_ty(e).peel_refs(), LangItem::String)
}

fn is_add(cx: &LateContext<'_>, src: &Expr<'_>, target: &Expr<'_>) -> bool {
    match peel_blocks(src).kind {
        ExprKind::Binary(
            Spanned {
                node: BinOpKind::Add, ..
            },
            left,
            _,
        ) => SpanlessEq::new(cx).eq_expr(target, left),
        _ => false,
    }
}
