use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_slice_of_primitives;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// When sorting primitive values (integers, bools, chars, as well
    /// as arrays, slices, and tuples of such items), it is typically better to
    /// use an unstable sort than a stable sort.
    ///
    /// ### Why is this bad?
    /// Typically, using a stable sort consumes more memory and cpu cycles.
    /// Because values which compare equal are identical, preserving their
    /// relative order (the guarantee that a stable sort provides) means
    /// nothing, while the extra costs still apply.
    ///
    /// ### Known problems
    ///
    /// As pointed out in
    /// [issue #8241](https://github.com/rust-lang/rust-clippy/issues/8241),
    /// a stable sort can instead be significantly faster for certain scenarios
    /// (eg. when a sorted vector is extended with new data and resorted).
    ///
    /// For more information and benchmarking results, please refer to the
    /// issue linked above.
    ///
    /// ### Example
    /// ```no_run
    /// let mut vec = vec![2, 1, 3];
    /// vec.sort();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut vec = vec![2, 1, 3];
    /// vec.sort_unstable();
    /// ```
    #[clippy::version = "1.47.0"]
    pub STABLE_SORT_PRIMITIVE,
    pedantic,
    "use of sort() when sort_unstable() is equivalent"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, recv: &'tcx Expr<'_>) {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(e.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
        && cx.tcx.type_of(impl_id).instantiate_identity().is_slice()
        && let Some(slice_type) = is_slice_of_primitives(cx, recv)
    {
        span_lint_and_then(
            cx,
            STABLE_SORT_PRIMITIVE,
            e.span,
            format!("used `sort` on primitive type `{slice_type}`"),
            |diag| {
                let mut app = Applicability::MachineApplicable;
                let recv_snip = snippet_with_context(cx, recv.span, e.span.ctxt(), "..", &mut app).0;
                diag.span_suggestion(e.span, "try", format!("{recv_snip}.sort_unstable()"), app);
                diag.note(
                    "an unstable sort typically performs faster without any observable difference for this data type",
                );
            },
        );
    }
}
