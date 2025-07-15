use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{path_to_local_id, peel_blocks, strip_pat_refs};
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, LetStmt, MatchSource, PatKind, QPath};
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for matches being used to destructure a single-variant enum
    /// or tuple struct where a `let` will suffice.
    ///
    /// ### Why is this bad?
    /// Just readability – `let` doesn't nest, whereas a `match` does.
    ///
    /// ### Example
    /// ```no_run
    /// enum Wrapper {
    ///     Data(i32),
    /// }
    ///
    /// let wrapper = Wrapper::Data(42);
    ///
    /// let data = match wrapper {
    ///     Wrapper::Data(i) => i,
    /// };
    /// ```
    ///
    /// The correct use would be:
    /// ```no_run
    /// enum Wrapper {
    ///     Data(i32),
    /// }
    ///
    /// let wrapper = Wrapper::Data(42);
    /// let Wrapper::Data(data) = wrapper;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub INFALLIBLE_DESTRUCTURING_MATCH,
    style,
    "a `match` statement with a single infallible arm instead of a `let`"
}

pub(crate) fn check(cx: &LateContext<'_>, local: &LetStmt<'_>) -> bool {
    if !local.span.from_expansion()
        && let Some(expr) = local.init
        && let ExprKind::Match(target, arms, MatchSource::Normal) = expr.kind
        && arms.len() == 1
        && arms[0].guard.is_none()
        && let PatKind::TupleStruct(QPath::Resolved(None, variant_name), args, _) = arms[0].pat.kind
        && args.len() == 1
        && let PatKind::Binding(binding, arg, ..) = strip_pat_refs(&args[0]).kind
        && let body = peel_blocks(arms[0].body)
        && path_to_local_id(body, arg)
    {
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            INFALLIBLE_DESTRUCTURING_MATCH,
            local.span,
            "you seem to be trying to use `match` to destructure a single infallible pattern. \
            Consider using `let`",
            "try",
            format!(
                "let {}({}{}) = {};",
                snippet_with_applicability(cx, variant_name.span, "..", &mut applicability),
                binding.prefix_str(),
                snippet_with_applicability(cx, local.pat.span, "..", &mut applicability),
                snippet_with_applicability(cx, target.span, "..", &mut applicability),
            ),
            applicability,
        );
        return true;
    }
    false
}
