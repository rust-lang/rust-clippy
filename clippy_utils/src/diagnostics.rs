//! Clippy wrappers around rustc's diagnostic functions.
//!
//! These functions are used by the `INTERNAL_METADATA_COLLECTOR` lint to collect the corresponding
//! lint applicability. Please make sure that you update the `LINT_EMISSION_FUNCTIONS` variable in
//! `clippy_lints::utils::internal_lints::metadata_collector` when a new function is added
//! or renamed.
//!
//! Thank you!
//! ~The `INTERNAL_METADATA_COLLECTOR` lint

use crate::is_in_test;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::{Applicability, Diag, DiagCtxtHandle, DiagMessage, Diagnostic, Level, MultiSpan};
#[cfg(debug_assertions)]
use rustc_errors::{EmissionGuarantee, SubstitutionPart, Suggestions};
use rustc_hir::HirId;
use rustc_lint::{EarlyContext, LateContext, Lint, LintContext};
use rustc_span::Span;
use std::env;
use std::sync::OnceLock;

/// The lints which the `allow-in-tests` configuration suppresses in test code, stored as
/// [`Lint::name`] values (e.g. `"clippy::UNWRAP_USED"`).
static ALLOWED_IN_TESTS: OnceLock<FxHashSet<&'static str>> = OnceLock::new();

/// Sets the lints which the `allow-in-tests` configuration suppresses in test code.
///
/// This must be called before any lint pass runs; only the first call has an effect.
pub fn set_lints_allowed_in_tests(lints: impl IntoIterator<Item = &'static str>) {
    let _ = ALLOWED_IN_TESTS.set(lints.into_iter().collect());
}

fn is_allowed_in_tests(lint: &'static Lint) -> bool {
    // This runs for every emitted lint, so check for the common case of an unset configuration
    // before hashing the lint's name.
    ALLOWED_IN_TESTS
        .get()
        .is_some_and(|lints| !lints.is_empty() && lints.contains(lint.name))
}

/// Extends [`LintContext`] with the information clippy's diagnostic functions need to apply the
/// `allow-in-tests` configuration.
pub trait ClippyLintContext: LintContext {
    /// Whether the node the lint level is resolved against is in test code, i.e. inside a
    /// `#[test]` function or a `#[cfg(test)]` item.
    fn is_in_test_code(&self) -> bool;
}

impl ClippyLintContext for LateContext<'_> {
    fn is_in_test_code(&self) -> bool {
        is_in_test(self.tcx, self.last_node_with_lint_attrs)
    }
}

/// Early lint passes run before the HIR is built, at which point test code becomes recognizable,
/// so `allow-in-tests` applies to late lint passes only.
impl ClippyLintContext for EarlyContext<'_> {
    fn is_in_test_code(&self) -> bool {
        false
    }
}

fn docs_link(diag: &mut Diag<'_, ()>, lint: &'static Lint) {
    if env::var("CLIPPY_DISABLE_DOCS_LINKS").is_err()
        && let Some(lint) = lint.name_lower().strip_prefix("clippy::")
    {
        diag.help(format!(
            "for further information visit https://rust-lang.github.io/rust-clippy/{}/index.html#{lint}",
            match option_env!("CFG_RELEASE_CHANNEL") {
                // Clippy version is 0.1.xx
                //
                // Always use .0 because we do not generate separate lint doc pages for rust patch releases
                Some("stable") => concat!("rust-1.", env!("CARGO_PKG_VERSION_PATCH"), ".0"),
                Some("beta") => "beta",
                _ => "master",
            }
        ));
    }
}

/// Makes sure that a diagnostic is well formed.
///
/// rustc debug asserts a few properties about spans,
/// but the clippy repo uses a distributed rustc build with debug assertions disabled,
/// so this has historically led to problems during subtree syncs where those debug assertions
/// only started triggered there.
///
/// This function makes sure we also validate them in debug clippy builds.
#[cfg(debug_assertions)]
fn validate_diag(diag: &Diag<'_, impl EmissionGuarantee>) {
    let suggestions = match &diag.suggestions {
        Suggestions::Enabled(suggs) => &**suggs,
        Suggestions::Sealed(suggs) => &**suggs,
        Suggestions::Disabled => return,
    };

    for substitution in suggestions.iter().flat_map(|s| &s.substitutions) {
        assert_eq!(
            substitution
                .parts
                .iter()
                .find(|SubstitutionPart { snippet, span }| snippet.is_empty() && span.is_empty()),
            None,
            "span must not be empty and have no suggestion"
        );

        assert_eq!(
            substitution
                .parts
                .array_windows()
                .find(|[a, b]| a.span.overlaps(b.span)),
            None,
            "suggestion must not have overlapping parts"
        );
    }
}

/// Emit a basic lint message with a `msg` and a `span`.
///
/// This is the most primitive of our lint emission methods and can
/// be a good way to get a new lint started.
///
/// Usually it's nicer to provide more context for lint messages.
/// Be sure the output is understandable when you use this method.
///
/// NOTE: Lint emissions are always bound to a node in the HIR, which is used to determine
/// the lint level.
/// For the `span_lint` function, the node that was passed into the `LintPass::check_*` function is
/// used.
///
/// If you're emitting the lint at the span of a different node than the one provided by the
/// `LintPass::check_*` function, consider using [`span_lint_hir`] instead.
/// This is needed for `#[allow]` and `#[expect]` attributes to work on the node
/// highlighted in the displayed warning.
///
/// If you're unsure which function you should use, you can test if the `#[expect]` attribute works
/// where you would expect it to.
/// If it doesn't, you likely need to use [`span_lint_hir`] instead.
///
/// # Example
///
/// ```ignore
/// error: usage of mem::forget on Drop type
///   --> tests/ui/mem_forget.rs:17:5
///    |
/// 17 |     std::mem::forget(seven);
///    |     ^^^^^^^^^^^^^^^^^^^^^^^
/// ```
#[track_caller]
pub fn span_lint<T: ClippyLintContext>(
    cx: &T,
    lint: &'static Lint,
    sp: impl Into<MultiSpan>,
    msg: impl Into<DiagMessage>,
) {
    span_lint_and_then(cx, lint, sp, msg, |_| {});
}

/// Same as [`span_lint`] but with an extra `help` message.
///
/// Use this if you want to provide some general help but
/// can't provide a specific machine applicable suggestion.
///
/// The `help` message can be optionally attached to a `Span`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// NOTE: Lint emissions are always bound to a node in the HIR, which is used to determine
/// the lint level.
/// For the `span_lint_and_help` function, the node that was passed into the `LintPass::check_*`
/// function is used.
///
/// If you're emitting the lint at the span of a different node than the one provided by the
/// `LintPass::check_*` function, consider using [`span_lint_hir_and_then`] instead.
/// This is needed for `#[allow]` and `#[expect]` attributes to work on the node
/// highlighted in the displayed warning.
///
/// If you're unsure which function you should use, you can test if the `#[expect]` attribute works
/// where you would expect it to.
/// If it doesn't, you likely need to use [`span_lint_hir_and_then`] instead.
///
/// # Example
///
/// ```text
/// error: constant division of 0.0 with 0.0 will always result in NaN
///   --> tests/ui/zero_div_zero.rs:6:25
///    |
/// 6  |     let other_f64_nan = 0.0f64 / 0.0;
///    |                         ^^^^^^^^^^^^
///    |
///    = help: consider using `f64::NAN` if you would like a constant representing NaN
/// ```
#[track_caller]
pub fn span_lint_and_help<T: ClippyLintContext>(
    cx: &T,
    lint: &'static Lint,
    span: impl Into<MultiSpan>,
    msg: impl Into<DiagMessage>,
    help_span: Option<Span>,
    help: impl Into<DiagMessage>,
) {
    span_lint_and_then(cx, lint, span, msg, |diag| {
        if let Some(help_span) = help_span {
            diag.span_help(help_span, help.into());
        } else {
            diag.help(help.into());
        }
    });
}

/// Like [`span_lint`] but with a `note` section instead of a `help` message.
///
/// The `note` message is presented separately from the main lint message
/// and is attached to a specific span:
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// NOTE: Lint emissions are always bound to a node in the HIR, which is used to determine
/// the lint level.
/// For the `span_lint_and_note` function, the node that was passed into the `LintPass::check_*`
/// function is used.
///
/// If you're emitting the lint at the span of a different node than the one provided by the
/// `LintPass::check_*` function, consider using [`span_lint_hir_and_then`] instead.
/// This is needed for `#[allow]` and `#[expect]` attributes to work on the node
/// highlighted in the displayed warning.
///
/// If you're unsure which function you should use, you can test if the `#[expect]` attribute works
/// where you would expect it to.
/// If it doesn't, you likely need to use [`span_lint_hir_and_then`] instead.
///
/// # Example
///
/// ```text
/// error: calls to `std::mem::forget` with a reference instead of an owned value. Forgetting a reference does nothing.
///   --> tests/ui/drop_forget_ref.rs:10:5
///    |
/// 10 |     forget(&SomeStruct);
///    |     ^^^^^^^^^^^^^^^^^^^
///    |
///    = note: `-D clippy::forget-ref` implied by `-D warnings`
/// note: argument has type &SomeStruct
///   --> tests/ui/drop_forget_ref.rs:10:12
///    |
/// 10 |     forget(&SomeStruct);
///    |            ^^^^^^^^^^^
/// ```
#[track_caller]
pub fn span_lint_and_note<T: ClippyLintContext>(
    cx: &T,
    lint: &'static Lint,
    span: impl Into<MultiSpan>,
    msg: impl Into<DiagMessage>,
    note_span: Option<Span>,
    note: impl Into<DiagMessage>,
) {
    span_lint_and_then(cx, lint, span, msg, |diag| {
        if let Some(note_span) = note_span {
            diag.span_note(note_span, note.into());
        } else {
            diag.note(note.into());
        }
    });
}

/// Like [`span_lint`] but allows to add notes, help and suggestions using a closure.
///
/// If you need to customize your lint output a lot, use this function.
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// NOTE: Lint emissions are always bound to a node in the HIR, which is used to determine
/// the lint level.
/// For the `span_lint_and_then` function, the node that was passed into the `LintPass::check_*`
/// function is used.
///
/// If you're emitting the lint at the span of a different node than the one provided by the
/// `LintPass::check_*` function, consider using [`span_lint_hir_and_then`] instead.
/// This is needed for `#[allow]` and `#[expect]` attributes to work on the node
/// highlighted in the displayed warning.
///
/// If you're unsure which function you should use, you can test if the `#[expect]` attribute works
/// where you would expect it to.
/// If it doesn't, you likely need to use [`span_lint_hir_and_then`] instead.
#[track_caller]
pub fn span_lint_and_then<C, S, M, F>(cx: &C, lint: &'static Lint, sp: S, msg: M, f: F)
where
    C: ClippyLintContext,
    S: Into<MultiSpan>,
    M: Into<DiagMessage>,
    F: FnOnce(&mut Diag<'_, ()>),
{
    struct ClippyDiag<F: FnOnce(&mut Diag<'_, ()>)>(F);

    impl<'a, F: FnOnce(&mut Diag<'_, ()>)> Diagnostic<'a, ()> for ClippyDiag<F> {
        fn into_diag(self, dcx: DiagCtxtHandle<'a>, level: Level) -> Diag<'a, ()> {
            let mut lint = Diag::new(dcx, level, "");
            (self.0)(&mut lint);
            lint
        }
    }

    if is_allowed_in_tests(lint) && cx.is_in_test_code() {
        return;
    }

    let sp = sp.into();
    #[expect(clippy::disallowed_methods)]
    cx.emit_span_lint(
        lint,
        sp.clone(),
        ClippyDiag(|diag: &mut Diag<'_, ()>| {
            diag.primary_message(msg);
            diag.span(sp);
            f(diag);
            docs_link(diag, lint);

            #[cfg(debug_assertions)]
            validate_diag(diag);
        }),
    );
}

/// Like [`span_lint`], but emits the lint at the node identified by the given `HirId`.
///
/// This is in contrast to [`span_lint`], which always emits the lint at the node that was last
/// passed to the `LintPass::check_*` function.
///
/// The `HirId` is used for checking lint level attributes and to fulfill lint expectations defined
/// via the `#[expect]` attribute.
///
/// For example:
/// ```ignore
/// fn f() { /* <node_1> */
///
///     #[allow(clippy::some_lint)]
///     let _x = /* <expr_1> */;
/// }
/// ```
/// If `some_lint` does its analysis in `LintPass::check_fn` (at `<node_1>`) and emits a lint at
/// `<expr_1>` using [`span_lint`], then allowing the lint at `<expr_1>` as attempted in the snippet
/// will not work!
/// Even though that is where the warning points at, which would be confusing to users.
///
/// Instead, use this function and also pass the `HirId` of `<expr_1>`, which will let
/// the compiler check lint level attributes at the place of the expression and
/// the `#[allow]` will work.
#[track_caller]
pub fn span_lint_hir(cx: &LateContext<'_>, lint: &'static Lint, hir_id: HirId, sp: Span, msg: impl Into<DiagMessage>) {
    span_lint_hir_and_then(cx, lint, hir_id, sp, msg, |_| {});
}

/// Like [`span_lint_and_then`], but emits the lint at the node identified by the given `HirId`.
///
/// This is in contrast to [`span_lint_and_then`], which always emits the lint at the node that was
/// last passed to the `LintPass::check_*` function.
///
/// The `HirId` is used for checking lint level attributes and to fulfill lint expectations defined
/// via the `#[expect]` attribute.
///
/// For example:
/// ```ignore
/// fn f() { /* <node_1> */
///
///     #[allow(clippy::some_lint)]
///     let _x = /* <expr_1> */;
/// }
/// ```
/// If `some_lint` does its analysis in `LintPass::check_fn` (at `<node_1>`) and emits a lint at
/// `<expr_1>` using [`span_lint`], then allowing the lint at `<expr_1>` as attempted in the snippet
/// will not work!
/// Even though that is where the warning points at, which would be confusing to users.
///
/// Instead, use this function and also pass the `HirId` of `<expr_1>`, which will let
/// the compiler check lint level attributes at the place of the expression and
/// the `#[allow]` will work.
#[track_caller]
pub fn span_lint_hir_and_then(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    hir_id: HirId,
    sp: impl Into<MultiSpan>,
    msg: impl Into<DiagMessage>,
    f: impl FnOnce(&mut Diag<'_, ()>),
) {
    if is_allowed_in_tests(lint) && is_in_test(cx.tcx, hir_id) {
        return;
    }

    #[expect(clippy::disallowed_methods)]
    cx.tcx.emit_node_span_lint(
        lint,
        hir_id,
        sp,
        rustc_errors::DiagDecorator(|diag| {
            diag.primary_message(msg);
            f(diag);
            docs_link(diag, lint);

            #[cfg(debug_assertions)]
            validate_diag(diag);
        }),
    );
}

/// Add a span lint with a suggestion on how to fix it.
///
/// These suggestions can be parsed by rustfix to allow it to automatically fix your code.
/// In the example below, `help` is `"try"` and `sugg` is the suggested replacement `".any(|x| x >
/// 2)"`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// NOTE: Lint emissions are always bound to a node in the HIR, which is used to determine
/// the lint level.
/// For the `span_lint_and_sugg` function, the node that was passed into the `LintPass::check_*`
/// function is used.
///
/// If you're emitting the lint at the span of a different node than the one provided by the
/// `LintPass::check_*` function, consider using [`span_lint_hir_and_then`] instead.
/// This is needed for `#[allow]` and `#[expect]` attributes to work on the node
/// highlighted in the displayed warning.
///
/// If you're unsure which function you should use, you can test if the `#[expect]` attribute works
/// where you would expect it to.
/// If it doesn't, you likely need to use [`span_lint_hir_and_then`] instead.
///
/// # Example
///
/// ```text
/// error: This `.fold` can be more succinctly expressed as `.any`
/// --> tests/ui/methods.rs:390:13
///     |
/// 390 |     let _ = (0..3).fold(false, |acc, x| acc || x > 2);
///     |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `.any(|x| x > 2)`
///     |
///     = note: `-D fold-any` implied by `-D warnings`
/// ```
#[track_caller]
pub fn span_lint_and_sugg<T: ClippyLintContext>(
    cx: &T,
    lint: &'static Lint,
    sp: Span,
    msg: impl Into<DiagMessage>,
    help: impl Into<DiagMessage>,
    sugg: String,
    applicability: Applicability,
) {
    span_lint_and_then(cx, lint, sp, msg.into(), |diag| {
        diag.span_suggestion(sp, help.into(), sugg, applicability);

        // This dummy construct is here to prevent the internal `clippy::collapsible_span_lint_calls`
        // lint from triggering. We don't want to allow/expect it as internal lints might or might
        // not be activated when linting, and we don't want an unknown lint warning either.
        std::hint::black_box(());
    });
}
