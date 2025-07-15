use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{method_chain_args, path_def_id, sym};
use rustc_errors::Applicability;
use rustc_lint::{LateContext, Lint};
use rustc_middle::ty;
use rustc_span::Symbol;
use {rustc_ast as ast, rustc_hir as hir};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.chars().next()` on a `str` to check
    /// if it starts with a given char.
    ///
    /// ### Why is this bad?
    /// Readability, this can be written more concisely as
    /// `_.starts_with(_)`.
    ///
    /// ### Example
    /// ```no_run
    /// let name = "foo";
    /// if name.chars().next() == Some('_') {};
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let name = "foo";
    /// if name.starts_with('_') {};
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub CHARS_NEXT_CMP,
    style,
    "using `.chars().next()` to check if a string starts with a char"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `_.chars().last()` or
    /// `_.chars().next_back()` on a `str` to check if it ends with a given char.
    ///
    /// ### Why is this bad?
    /// Readability, this can be written more concisely as
    /// `_.ends_with(_)`.
    ///
    /// ### Example
    /// ```no_run
    /// # let name = "_";
    /// name.chars().last() == Some('_') || name.chars().next_back() == Some('-');
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let name = "_";
    /// name.ends_with('_') || name.ends_with('-');
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub CHARS_LAST_CMP,
    style,
    "using `.chars().last()` or `.chars().next_back()` to check if a string ends with a char"
}

/// Checks for the `CHARS_NEXT_CMP` lint.
pub(super) fn check_next(cx: &LateContext<'_>, info: &crate::BinaryExprInfo<'_>) -> bool {
    check(cx, info, &[sym::chars, sym::next], CHARS_NEXT_CMP, "starts_with")
}

/// Checks for the `CHARS_NEXT_CMP` lint with `unwrap()`.
pub(super) fn check_next_unwrap(cx: &LateContext<'_>, info: &crate::BinaryExprInfo<'_>) -> bool {
    check_unwrap(
        cx,
        info,
        &[sym::chars, sym::next, sym::unwrap],
        CHARS_NEXT_CMP,
        "starts_with",
    )
}

/// Checks for the `CHARS_LAST_CMP` lint.
pub(super) fn check_last(cx: &LateContext<'_>, info: &crate::BinaryExprInfo<'_>) -> bool {
    if check(cx, info, &[sym::chars, sym::last], CHARS_LAST_CMP, "ends_with") {
        true
    } else {
        check(cx, info, &[sym::chars, sym::next_back], CHARS_LAST_CMP, "ends_with")
    }
}

/// Checks for the `CHARS_LAST_CMP` lint with `unwrap()`.
pub(super) fn check_last_unwrap(cx: &LateContext<'_>, info: &crate::BinaryExprInfo<'_>) -> bool {
    if check_unwrap(
        cx,
        info,
        &[sym::chars, sym::last, sym::unwrap],
        CHARS_LAST_CMP,
        "ends_with",
    ) {
        true
    } else {
        check_unwrap(
            cx,
            info,
            &[sym::chars, sym::next_back, sym::unwrap],
            CHARS_LAST_CMP,
            "ends_with",
        )
    }
}

/// Wrapper fn for `CHARS_NEXT_CMP` and `CHARS_LAST_CMP` lints.
fn check(
    cx: &LateContext<'_>,
    info: &crate::BinaryExprInfo<'_>,
    chain_methods: &[Symbol],
    lint: &'static Lint,
    suggest: &str,
) -> bool {
    if let Some(args) = method_chain_args(info.chain, chain_methods)
        && let hir::ExprKind::Call(fun, [arg_char]) = info.other.kind
        && let Some(id) = path_def_id(cx, fun).map(|ctor_id| cx.tcx.parent(ctor_id))
        && Some(id) == cx.tcx.lang_items().option_some_variant()
    {
        let mut applicability = Applicability::MachineApplicable;
        let self_ty = cx.typeck_results().expr_ty_adjusted(args[0].0).peel_refs();

        if *self_ty.kind() != ty::Str {
            return false;
        }

        span_lint_and_sugg(
            cx,
            lint,
            info.expr.span,
            format!("you should use the `{suggest}` method"),
            "like this",
            format!(
                "{}{}.{suggest}({})",
                if info.eq { "" } else { "!" },
                snippet_with_applicability(cx, args[0].0.span, "..", &mut applicability),
                snippet_with_applicability(cx, arg_char.span, "..", &mut applicability)
            ),
            applicability,
        );

        return true;
    }

    false
}

/// Wrapper fn for `CHARS_NEXT_CMP` and `CHARS_LAST_CMP` lints with `unwrap()`.
fn check_unwrap(
    cx: &LateContext<'_>,
    info: &crate::BinaryExprInfo<'_>,
    chain_methods: &[Symbol],
    lint: &'static Lint,
    suggest: &str,
) -> bool {
    if let Some(args) = method_chain_args(info.chain, chain_methods)
        && let hir::ExprKind::Lit(lit) = info.other.kind
        && let ast::LitKind::Char(c) = lit.node
    {
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            lint,
            info.expr.span,
            format!("you should use the `{suggest}` method"),
            "like this",
            format!(
                "{}{}.{suggest}('{}')",
                if info.eq { "" } else { "!" },
                snippet_with_applicability(cx, args[0].0.span, "..", &mut applicability),
                c.escape_default()
            ),
            applicability,
        );

        true
    } else {
        false
    }
}
