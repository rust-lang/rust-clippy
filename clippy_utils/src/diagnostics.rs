//! Clippy wrappers around rustc's diagnostic functions.
//!
//! These functions are used by the `INTERNAL_METADATA_COLLECTOR` lint to collect the corresponding
//! lint applicability. Please make sure that you update the `LINT_EMISSION_FUNCTIONS` variable in
//! `clippy_lints::utils::internal_lints::metadata_collector` when a new function is added
//! or renamed.
//!
//! Thank you!
//! ~The `INTERNAL_METADATA_COLLECTOR` lint

use rustc_errors::{emitter::MAX_SUGGESTION_HIGHLIGHT_LINES, Applicability, Diagnostic};
use rustc_hir::HirId;
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_span::source_map::{MultiSpan, Span};
use std::env;

fn docs_link(diag: &mut Diagnostic, lint: &'static Lint) {
    if env::var("CLIPPY_DISABLE_DOCS_LINKS").is_err() {
        if let Some(lint) = lint.name_lower().strip_prefix("clippy::") {
            diag.help(&format!(
                "for further information visit https://rust-lang.github.io/rust-clippy/{}/index.html#{}",
                &option_env!("RUST_RELEASE_NUM").map_or("master".to_string(), |n| {
                    // extract just major + minor version and ignore patch versions
                    format!("rust-{}", n.rsplit_once('.').unwrap().1)
                }),
                lint
            ));
        }
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
/// # Example
///
/// ```ignore
/// error: usage of mem::forget on Drop type
///   --> $DIR/mem_forget.rs:17:5
///    |
/// 17 |     std::mem::forget(seven);
///    |     ^^^^^^^^^^^^^^^^^^^^^^^
/// ```
pub fn span_lint<T: LintContext>(cx: &T, lint: &'static Lint, sp: impl Into<MultiSpan>, msg: &str) {
    cx.struct_span_lint(lint, sp, |diag| {
        let mut diag = diag.build(msg);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Same as `span_lint` but with an extra `help` message.
///
/// Use this if you want to provide some general help but
/// can't provide a specific machine applicable suggestion.
///
/// The `help` message can be optionally attached to a `Span`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// # Example
///
/// ```text
/// error: constant division of 0.0 with 0.0 will always result in NaN
///   --> $DIR/zero_div_zero.rs:6:25
///    |
/// 6  |     let other_f64_nan = 0.0f64 / 0.0;
///    |                         ^^^^^^^^^^^^
///    |
///    = help: consider using `f64::NAN` if you would like a constant representing NaN
/// ```
pub fn span_lint_and_help<'a, T: LintContext>(
    cx: &'a T,
    lint: &'static Lint,
    span: Span,
    msg: &str,
    help_span: Option<Span>,
    help: &str,
) {
    cx.struct_span_lint(lint, span, |diag| {
        let mut diag = diag.build(msg);
        if let Some(help_span) = help_span {
            diag.span_help(help_span, help);
        } else {
            diag.help(help);
        }
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Like `span_lint` but with a `note` section instead of a `help` message.
///
/// The `note` message is presented separately from the main lint message
/// and is attached to a specific span:
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// # Example
///
/// ```text
/// error: calls to `std::mem::forget` with a reference instead of an owned value. Forgetting a reference does nothing.
///   --> $DIR/drop_forget_ref.rs:10:5
///    |
/// 10 |     forget(&SomeStruct);
///    |     ^^^^^^^^^^^^^^^^^^^
///    |
///    = note: `-D clippy::forget-ref` implied by `-D warnings`
/// note: argument has type &SomeStruct
///   --> $DIR/drop_forget_ref.rs:10:12
///    |
/// 10 |     forget(&SomeStruct);
///    |            ^^^^^^^^^^^
/// ```
pub fn span_lint_and_note<'a, T: LintContext>(
    cx: &'a T,
    lint: &'static Lint,
    span: impl Into<MultiSpan>,
    msg: &str,
    note_span: Option<Span>,
    note: &str,
) {
    cx.struct_span_lint(lint, span, |diag| {
        let mut diag = diag.build(msg);
        if let Some(note_span) = note_span {
            diag.span_note(note_span, note);
        } else {
            diag.note(note);
        }
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Like `span_lint` but allows to add notes, help and suggestions using a closure.
///
/// If you need to customize your lint output a lot, use this function.
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
pub fn span_lint_and_then<C, S, F>(cx: &C, lint: &'static Lint, sp: S, msg: &str, f: F)
where
    C: LintContext,
    S: Into<MultiSpan>,
    F: FnOnce(&mut Diagnostic),
{
    cx.struct_span_lint(lint, sp, |diag| {
        let mut diag = diag.build(msg);
        f(&mut diag);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

pub fn span_lint_hir(cx: &LateContext<'_>, lint: &'static Lint, hir_id: HirId, sp: Span, msg: &str) {
    cx.tcx.struct_span_lint_hir(lint, hir_id, sp, |diag| {
        let mut diag = diag.build(msg);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

pub fn span_lint_hir_and_then(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    hir_id: HirId,
    sp: impl Into<MultiSpan>,
    msg: &str,
    f: impl FnOnce(&mut Diagnostic),
) {
    cx.tcx.struct_span_lint_hir(lint, hir_id, sp, |diag| {
        let mut diag = diag.build(msg);
        f(&mut diag);
        docs_link(&mut diag, lint);
        diag.emit();
    });
}

/// Add a span lint with a suggestion on how to fix it.
///
/// These suggestions can be parsed by rustfix to allow it to automatically fix your code.
/// In the example below, `help` is `"try"` and `sugg` is the suggested replacement `".any(|x| x >
/// 2)"`.
///
/// If you change the signature, remember to update the internal lint `CollapsibleCalls`
///
/// # Example
///
/// ```text
/// error: This `.fold` can be more succinctly expressed as `.any`
/// --> $DIR/methods.rs:390:13
///     |
/// 390 |     let _ = (0..3).fold(false, |acc, x| acc || x > 2);
///     |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: try: `.any(|x| x > 2)`
///     |
///     = note: `-D fold-any` implied by `-D warnings`
/// ```
#[cfg_attr(feature = "internal", allow(clippy::collapsible_span_lint_calls))]
pub fn span_lint_and_sugg<'a, T: LintContext>(
    cx: &'a T,
    lint: &'static Lint,
    sp: Span,
    msg: &str,
    help: &str,
    sugg: String,
    applicability: Applicability,
) {
    span_lint_and_then(cx, lint, sp, msg, |diag| {
        diag.span_suggestion(sp, help, sugg, applicability);
    });
}

/// Like [`span_lint_and_sugg`] with a focus on the edges. The output will either
/// emit single span or multispan suggestion depending on the number of its lines.
///
/// If the given suggestion string has more lines than the maximum display length defined by
/// [`MAX_SUGGESTION_HIGHLIGHT_LINES`][`rustc_errors::emitter::MAX_SUGGESTION_HIGHLIGHT_LINES`],
/// this function will split the suggestion and span to showcase the change for the top and
/// bottom edge of the code. For normal suggestions, in one display window, the help message
/// will be combined with a colon.
///
/// Multipart suggestions like the one being created here currently cannot be
/// applied by rustfix (See [rustfix#141](https://github.com/rust-lang/rustfix/issues/141)).
/// Testing rustfix with this lint emission function might require a file with
/// suggestions that can be fixed and those that can't. See
/// [clippy#8520](https://github.com/rust-lang/rust-clippy/pull/8520/files) for
/// an example and of this.
///
/// # Example for a long suggestion
///
/// ```text
/// error: called `map(..).flatten()` on `Option`
///   --> $DIR/map_flatten.rs:8:10
///    |
/// LL |           .map(|x| {
///    |  __________^
/// LL | |             if x <= 5 {
/// LL | |                 Some(x)
/// LL | |             } else {
/// ...  |
/// LL | |         })
/// LL | |         .flatten();
///    | |__________________^
///    |
///   = note: `-D clippy::map-flatten` implied by `-D warnings`
/// help: try replacing `map` with `and_then`
///    |
/// LL ~         .and_then(|x| {
/// LL +             if x <= 5 {
/// LL +                 Some(x)
///    |
/// help: and remove the `.flatten()`
///    |
/// LL +                 None
/// LL +             }
/// LL ~         });
///    |
/// ```
pub fn span_lint_and_sugg_for_edges(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    sp: Span,
    msg: &str,
    helps: &[&str; 2],
    sugg: String,
    applicability: Applicability,
) {
    span_lint_and_then(cx, lint, sp, msg, |diag| {
        let sugg_lines_count = sugg.lines().count();
        if sugg_lines_count > MAX_SUGGESTION_HIGHLIGHT_LINES {
            let sm = cx.sess().source_map();
            if let (Ok(line_upper), Ok(line_bottom)) = (sm.lookup_line(sp.lo()), sm.lookup_line(sp.hi())) {
                let split_idx = MAX_SUGGESTION_HIGHLIGHT_LINES / 2;
                let span_upper = sm.span_until_char(sp.with_hi(line_upper.sf.lines[line_upper.line + split_idx]), '\n');
                let span_bottom = sp.with_lo(line_bottom.sf.lines[line_bottom.line - split_idx]);

                let sugg_lines_vec = sugg.lines().collect::<Vec<&str>>();
                let sugg_upper = sugg_lines_vec[..split_idx].join("\n");
                let sugg_bottom = sugg_lines_vec[sugg_lines_count - split_idx..].join("\n");

                diag.span_suggestion(span_upper, helps[0], sugg_upper, applicability);
                diag.span_suggestion(span_bottom, helps[1], sugg_bottom, applicability);

                return;
            }
        }
        diag.span_suggestion_with_style(
            sp,
            &helps.join(", "),
            sugg,
            applicability,
            rustc_errors::SuggestionStyle::ShowAlways,
        );
    });
}

/// Create a suggestion made from several `span → replacement`.
///
/// Note: in the JSON format (used by `compiletest_rs`), the help message will
/// appear once per
/// replacement. In human-readable format though, it only appears once before
/// the whole suggestion.
pub fn multispan_sugg<I>(diag: &mut Diagnostic, help_msg: &str, sugg: I)
where
    I: IntoIterator<Item = (Span, String)>,
{
    multispan_sugg_with_applicability(diag, help_msg, Applicability::Unspecified, sugg);
}

/// Create a suggestion made from several `span → replacement`.
///
/// rustfix currently doesn't support the automatic application of suggestions with
/// multiple spans. This is tracked in issue [rustfix#141](https://github.com/rust-lang/rustfix/issues/141).
/// Suggestions with multiple spans will be silently ignored.
pub fn multispan_sugg_with_applicability<I>(
    diag: &mut Diagnostic,
    help_msg: &str,
    applicability: Applicability,
    sugg: I,
) where
    I: IntoIterator<Item = (Span, String)>,
{
    diag.multipart_suggestion(help_msg, sugg.into_iter().collect(), applicability);
}
