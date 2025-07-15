use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_diag_trait_item;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty::print::with_forced_trimmed_paths;
use rustc_middle::ty::{self};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of `_.to_owned()`, on a `Cow<'_, _>`.
    ///
    /// ### Why is this bad?
    /// Calling `to_owned()` on a `Cow` creates a clone of the `Cow`
    /// itself, without taking ownership of the `Cow` contents (i.e.
    /// it's equivalent to calling `Cow::clone`).
    /// The similarly named `into_owned` method, on the other hand,
    /// clones the `Cow` contents, effectively turning any `Cow::Borrowed`
    /// into a `Cow::Owned`.
    ///
    /// Given the potential ambiguity, consider replacing `to_owned`
    /// with `clone` for better readability or, if getting a `Cow::Owned`
    /// was the original intent, using `into_owned` instead.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::borrow::Cow;
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// let data = cow.to_owned();
    /// assert!(matches!(data, Cow::Borrowed(_)))
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::borrow::Cow;
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// let data = cow.clone();
    /// assert!(matches!(data, Cow::Borrowed(_)))
    /// ```
    /// or
    /// ```no_run
    /// # use std::borrow::Cow;
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// let _data: String = cow.into_owned();
    /// ```
    #[clippy::version = "1.65.0"]
    pub SUSPICIOUS_TO_OWNED,
    suspicious,
    "calls to `to_owned` on a `Cow<'_, _>` might not do what they are expected"
}

pub fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) -> bool {
    if let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && is_diag_trait_item(cx, method_def_id, sym::ToOwned)
        && let input_type = cx.typeck_results().expr_ty(expr)
        && let ty::Adt(adt, _) = cx.typeck_results().expr_ty(expr).kind()
        && cx.tcx.is_diagnostic_item(sym::Cow, adt.did())
    {
        let mut app = Applicability::MaybeIncorrect;
        let recv_snip = snippet_with_context(cx, recv.span, expr.span.ctxt(), "..", &mut app).0;
        span_lint_and_then(
            cx,
            SUSPICIOUS_TO_OWNED,
            expr.span,
            with_forced_trimmed_paths!(format!(
                "this `to_owned` call clones the {input_type} itself and does not cause the {input_type} contents to become owned"
            )),
            |diag| {
                diag.span_suggestion(
                    expr.span,
                    "depending on intent, either make the Cow an Owned variant",
                    format!("{recv_snip}.into_owned()"),
                    app,
                );
                diag.span_suggestion(
                    expr.span,
                    "or clone the Cow itself",
                    format!("{recv_snip}.clone()"),
                    app,
                );
            },
        );
        return true;
    }
    false
}
