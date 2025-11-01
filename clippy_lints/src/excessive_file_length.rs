use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LOCAL_CRATE;
use rustc_span::{FileName, Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for files that exceed a configurable line count threshold.
    ///
    /// Note: This is a restriction lint that is allow-by-default. You need to enable it
    /// explicitly in your configuration and set the threshold in clippy.toml.
    ///
    /// ### Why restrict this?
    /// Large files can be harder to navigate and understand. They often indicate that
    /// the code could benefit from being split into multiple smaller, more focused modules.
    /// This improves maintainability and makes the codebase easier to understand.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// excessive-file-length-threshold = 500
    /// ```
    ///
    /// If a file exceeds this threshold, the lint will suggest splitting it into
    /// smaller modules.
    #[clippy::version = "1.84.0"]
    pub EXCESSIVE_FILE_LENGTH,
    restriction,
    "checks for files that exceed a configurable line count threshold"
}

impl_lint_pass!(ExcessiveFileLength => [EXCESSIVE_FILE_LENGTH]);

pub struct ExcessiveFileLength {
    pub excessive_file_length_threshold: u64,
}

impl ExcessiveFileLength {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            excessive_file_length_threshold: conf.excessive_file_length_threshold,
        }
    }
}

impl EarlyLintPass for ExcessiveFileLength {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &Crate) {
        // Only check if threshold is set (non-zero)
        if self.excessive_file_length_threshold == 0 {
            return;
        }

        // Get all source files for the local crate
        let source_map = cx.sess().source_map();

        // We want to check each file in the current crate
        for file in source_map.files().iter() {
            // Only check files from the local crate, not external dependencies
            if file.cnum != LOCAL_CRATE {
                continue;
            }

            // Skip non-real files (generated code, etc.)
            if !matches!(&file.name, FileName::Real(_)) {
                continue;
            }

            // Count total lines in the file
            let line_count = file.count_lines() as u64;

            // Check if file exceeds threshold
            if line_count > self.excessive_file_length_threshold {
                // Create a span at the start of the file for the lint
                let span = Span::new(
                    file.start_pos,
                    file.start_pos,
                    SyntaxContext::root(),
                    None,
                );

                span_lint_and_help(
                    cx,
                    EXCESSIVE_FILE_LENGTH,
                    span,
                    format!(
                        "this file has too many lines ({}/{})",
                        line_count, self.excessive_file_length_threshold
                    ),
                    None,
                    "consider splitting this file into smaller modules to improve maintainability",
                );
            }
        }
    }
}
