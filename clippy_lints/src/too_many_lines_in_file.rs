use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint;
use rustc_ast::ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LOCAL_CRATE;
use rustc_span::{FileName, Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for source files that have a large number of lines of code.
    /// Blank lines and lines containing only comments are not counted.
    ///
    /// ### Why restrict this?
    /// Large files are harder to navigate and understand. They often indicate
    /// that a module has too many responsibilities and should be split into
    /// smaller, more focused modules.
    ///
    /// ### Example
    /// A file with more lines than the configured threshold will trigger this lint.
    /// The fix is to split it into smaller modules.
    ///
    /// ### Configuration
    /// The maximum number of lines is configured with `too-many-lines-in-file-threshold`
    /// (default: `1000`).
    #[clippy::version = "1.97.0"]
    pub TOO_MANY_LINES_IN_FILE,
    restriction,
    "files with too many lines of code"
}

impl_lint_pass!(TooManyLinesInFile => [TOO_MANY_LINES_IN_FILE]);

pub struct TooManyLinesInFile {
    threshold: u64,
}

impl TooManyLinesInFile {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            threshold: conf.too_many_lines_in_file_threshold,
        }
    }
}

impl EarlyLintPass for TooManyLinesInFile {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &ast::Crate) {
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            if file.cnum != LOCAL_CRATE {
                continue;
            }
            if !matches!(file.name, FileName::Real(_)) {
                continue;
            }
            let Some(src) = file.src.as_deref() else {
                continue;
            };

            let mut line_count: u64 = 0;
            let mut in_comment = false;
            let mut threshold_exceeded_offset: Option<u32> = None;
            let mut byte_offset: u32 = 0;

            for mut line in src.lines() {
                let line_start_offset = byte_offset;
                byte_offset = byte_offset
                    .saturating_add(u32::try_from(line.len()).unwrap_or(u32::MAX))
                    .saturating_add(1); // +1 for newline
                let mut code_in_line = false;
                loop {
                    line = line.trim_start();
                    if line.is_empty() {
                        break;
                    }
                    if in_comment {
                        if let Some(i) = line.find("*/") {
                            line = &line[i + 2..];
                            in_comment = false;
                            continue;
                        }
                    } else {
                        let multi_idx = line.find("/*").unwrap_or(line.len());
                        let single_idx = line.find("//").unwrap_or(line.len());
                        code_in_line |= multi_idx > 0 && single_idx > 0;
                        if multi_idx < single_idx {
                            line = &line[multi_idx + 2..];
                            in_comment = true;
                            continue;
                        }
                    }
                    break;
                }
                if code_in_line {
                    line_count += 1;
                    if line_count == self.threshold + 1 {
                        threshold_exceeded_offset = Some(line_start_offset);
                    }
                }
            }

            if line_count > self.threshold {
                let start = file.start_pos + rustc_span::BytePos(threshold_exceeded_offset.unwrap_or(0));
                let span = Span::new(start, start, SyntaxContext::root(), None);
                span_lint(
                    cx,
                    TOO_MANY_LINES_IN_FILE,
                    span,
                    format!("this file has too many lines ({line_count}/{})", self.threshold),
                );
            }
        }
    }
}
