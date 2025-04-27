//! JSON output and comparison functionality for Clippy warnings.
//!
//! This module handles serialization of Clippy warnings to JSON format,
//! loading warnings from JSON files, and generating human-readable diffs
//! between different linting runs.

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use itertools::{EitherOrBoth, Itertools};
use serde::{Deserialize, Serialize};

use crate::ClippyWarning;

/// This is the total number. 300 warnings results in 100 messages per section.
const DEFAULT_LIMIT_PER_LINT: usize = 300;
/// Target for total warnings to display across all lints when truncating output.
const TRUNCATION_TOTAL_TARGET: usize = 1000;

#[derive(Debug, Deserialize, Serialize)]
struct LintJson {
    /// The lint name e.g. `clippy::bytes_nth`
    name: String,
    /// The filename and line number e.g. `anyhow-1.0.86/src/error.rs:42`
    file_line: String,
    file_url: String,
    rendered: String,
}

impl LintJson {
    fn key(&self) -> impl Ord + '_ {
        (self.name.as_str(), self.file_line.as_str())
    }

    fn format_info_text(&self, out: &mut String, action: &str) {
        let _ = writeln!(
            out,
            "{action} `{}` at [`{}`]({})",
            self.name, self.file_line, self.file_url
        );
    }
}

/// Creates the log file output for [`crate::config::OutputFormat::Json`]
pub(crate) fn output(clippy_warnings: Vec<ClippyWarning>) -> String {
    let mut lints: Vec<LintJson> = clippy_warnings
        .into_iter()
        .map(|warning| {
            let span = warning.span();
            let file_name = span
                .file_name
                .strip_prefix("target/lintcheck/sources/")
                .unwrap_or(&span.file_name);
            let file_line = format!("{file_name}:{}", span.line_start);
            LintJson {
                name: warning.name,
                file_line,
                file_url: warning.url,
                rendered: warning.diag.rendered.unwrap().trim().to_string(),
            }
        })
        .collect();
    lints.sort_by(|a, b| a.key().cmp(&b.key()));
    serde_json::to_string(&lints).unwrap()
}

/// Loads lint warnings from a JSON file at the given path.
fn load_warnings(path: &Path) -> Vec<LintJson> {
    let file = fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    serde_json::from_slice(&file).unwrap_or_else(|e| panic!("failed to deserialize {}: {e}", path.display()))
}

/// Generates and prints a diff between two sets of lint warnings.
///
/// Compares warnings from `old_path` and `new_path`, then displays a summary table
/// and detailed information about added, removed, and changed warnings.
pub(crate) fn diff(old_path: &Path, new_path: &Path, truncate: bool, output: Option<PathBuf>) {
    let old_warnings = load_warnings(old_path);
    let new_warnings = load_warnings(new_path);

    let mut lint_warnings = vec![];

    for (name, changes) in &itertools::merge_join_by(old_warnings, new_warnings, |old, new| old.key().cmp(&new.key()))
        .chunk_by(|change| change.as_ref().into_left().name.clone())
    {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        for change in changes {
            match change {
                EitherOrBoth::Both(old, new) => {
                    if old.rendered != new.rendered {
                        changed.push((old, new));
                    }
                },
                EitherOrBoth::Left(old) => removed.push(old),
                EitherOrBoth::Right(new) => added.push(new),
            }
        }

        if !added.is_empty() || !removed.is_empty() || !changed.is_empty() {
            lint_warnings.push(LintWarnings {
                name,
                added,
                removed,
                changed,
            });
        }
    }

    if lint_warnings.is_empty() {
        return;
    }

    let summary = format_summary(&lint_warnings);

    let truncate_after = if truncate {
        // Max 15 ensures that we at least have five messages per lint
        DEFAULT_LIMIT_PER_LINT
            .min(TRUNCATION_TOTAL_TARGET / lint_warnings.len())
            .max(15)
    } else {
        // No lint should ever each this number of lint emissions, so this is equivialent to
        // No truncation
        usize::MAX
    };

    let mut diff = summary.clone();
    for lint in lint_warnings {
        format_lint_warnings(&mut diff, &lint, truncate_after);
    }

    if let Some(output) = output {
        print!("{summary}");
        fs::write(output, diff).unwrap();
    } else {
        print!("{diff}");
    }
}

/// Container for grouped lint warnings organized by status (added/removed/changed).
#[derive(Debug)]
struct LintWarnings {
    name: String,
    added: Vec<LintJson>,
    removed: Vec<LintJson>,
    changed: Vec<(LintJson, LintJson)>,
}

fn format_lint_warnings(out: &mut String, lint: &LintWarnings, truncate_after: usize) {
    let name = &lint.name;
    let html_id = to_html_id(name);

    let _ = writeln!(out, r#"## `{name}` <a id="user-content-{html_id}"></a>"#);

    let _ = writeln!(
        out,
        r"{}, {}, {}",
        count_string(name, "added", lint.added.len()),
        count_string(name, "removed", lint.removed.len()),
        count_string(name, "changed", lint.changed.len()),
    );

    format_warnings(out, "Added", &lint.added, truncate_after / 3);
    format_warnings(out, "Removed", &lint.removed, truncate_after / 3);
    format_changed_diff(out, &lint.changed, truncate_after / 3);
}

fn format_summary(lints: &[LintWarnings]) -> String {
    let mut summary = "\
| Lint | Added | Removed | Changed |
| ---- | ----: | ------: | ------: |
"
    .to_string();

    // Create an absolute URL when running under github actions so the summary can be posted as a
    // comment
    let base_url = if let Ok(repo) = env::var("GITHUB_REPOSITORY")
        && let Ok(run_id) = env::var("GITHUB_RUN_ID")
    {
        format!("https://github.com/{repo}/actions/runs/{run_id}")
    } else {
        String::new()
    };

    for lint in lints {
        let _ = writeln!(
            &mut summary,
            "| [`{}`]({base_url}#user-content-{}) | {} | {} | {} |",
            lint.name,
            to_html_id(&lint.name),
            lint.added.len(),
            lint.removed.len(),
            lint.changed.len()
        );
    }

    summary
}

fn format_warnings(out: &mut String, title: &str, warnings: &[LintJson], truncate_after: usize) {
    let Some(first) = warnings.first() else { return };

    format_h3(out, &first.name, title);

    let warnings = truncate(out, warnings, truncate_after);

    for warning in warnings {
        warning.format_info_text(out, title);
        let _ = writeln!(out, "```\n{}\n```", warning.rendered);
    }
}

fn format_changed_diff(out: &mut String, changed: &[(LintJson, LintJson)], truncate_after: usize) {
    let Some((first_old, _)) = changed.first() else { return };

    format_h3(out, &first_old.name, "Changed");

    let changed = truncate(out, changed, truncate_after);

    for (old, new) in changed {
        new.format_info_text(out, "Changed");
        let _ = writeln!(out, "```diff");
        for change in diff::lines(&old.rendered, &new.rendered) {
            use diff::Result::{Both, Left, Right};

            let _ = match change {
                Both(unchanged, _) => writeln!(out, " {unchanged}"),
                Left(removed) => writeln!(out, "-{removed}"),
                Right(added) => writeln!(out, "+{added}"),
            };
        }
        let _ = writeln!(out, "```");
    }
}

fn truncate<'a, T>(out: &mut String, list: &'a [T], truncate_after: usize) -> &'a [T] {
    if list.len() > truncate_after {
        let _ = writeln!(
            out,
            "{} warnings have been truncated for this summary.\n",
            list.len() - truncate_after
        );

        list.split_at(truncate_after).0
    } else {
        list
    }
}

fn format_h3(out: &mut String, lint: &str, title: &str) {
    let html_id = to_html_id(lint);
    // We have to use HTML here to be able to manually add an id.
    let _ = writeln!(out, r#"### {title} <a id="user-content-{html_id}-{title}"></a>"#);
}

/// GitHub's markdown parsers doesn't like IDs with `::` and `_`. This simplifies
/// the lint name for the HTML ID.
fn to_html_id(lint_name: &str) -> String {
    lint_name.replace("clippy::", "").replace('_', "-")
}

/// This generates the `x added` string for the start of the lint summary.
/// It linkifies them if possible to jump to the respective heading.
fn count_string(lint: &str, label: &str, count: usize) -> String {
    // Headlines are only added, if anything will be displayed under the headline.
    // We therefore only want to add links to them if they exist
    if count == 0 {
        format!("0 {label}")
    } else {
        let html_id = to_html_id(lint);
        // GitHub's job summaries don't add HTML ids to headings. That's why we
        // manually have to add them above. User supplied IDs always start with
        // `user-content-`
        format!("[{count} {label}](#user-content-{html_id}-{label})")
    }
}
