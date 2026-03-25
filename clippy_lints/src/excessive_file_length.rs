use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::tokenize_with_text;
use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::{HirId, Mod};
use rustc_lexer::TokenKind;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LOCAL_CRATE;
use rustc_span::{FileName, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for source files that exceed a configurable number of lines of code.
    ///
    /// ### Why restrict this?
    /// Long source files can be difficult to navigate, understand, and maintain.
    /// Splitting them into smaller, focused modules encourages better code
    /// organization.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// excessive-file-length-threshold = 500
    /// ```
    ///
    /// A file exceeding the threshold should be refactored by extracting
    /// logical sections into separate modules.
    #[clippy::version = "1.96.0"]
    pub EXCESSIVE_FILE_LENGTH,
    restriction,
    "source file exceeds a configurable line count"
}

impl_lint_pass!(ExcessiveFileLength => [EXCESSIVE_FILE_LENGTH]);

struct FileInfo {
    hir_id: HirId,
    span: Span,
    code_lines: u64,
}

pub struct ExcessiveFileLength {
    threshold: u64,
    files: FxIndexMap<FileName, FileInfo>,
}

impl ExcessiveFileLength {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            threshold: conf.excessive_file_length_threshold,
            files: FxIndexMap::default(),
        }
    }
}

/// Counts lines that contain at least one code token, skipping lines that are
/// blank or only contain comments. Uses `rustc_lexer` for correct tokenization.
fn count_code_lines(src: &str) -> u64 {
    let mut lines_with_code = vec![false; src.lines().count()];
    let mut current_line = 0;

    for (kind, text, _) in tokenize_with_text(src) {
        let is_code = !matches!(
            kind,
            TokenKind::Whitespace
                | TokenKind::LineComment { .. }
                | TokenKind::BlockComment { .. }
        );

        if is_code {
            lines_with_code[current_line] = true;
        }

        current_line += text.bytes().filter(|&b| b == b'\n').count();
    }

    lines_with_code.iter().filter(|&&has_code| has_code).count() as u64
}

impl<'tcx> LateLintPass<'tcx> for ExcessiveFileLength {
    fn check_mod(&mut self, cx: &LateContext<'_>, module: &Mod<'_>, hir_id: HirId) {
        if self.threshold == 0 {
            return;
        }

        let span = module.spans.inner_span;
        if span.from_expansion() {
            return;
        }

        let source_map = cx.sess().source_map();
        let file = source_map.lookup_source_file(span.lo());
        if file.cnum != LOCAL_CRATE {
            return;
        }

        let name = file.name.clone();
        if !matches!(name, FileName::Real(_)) {
            return;
        }

        if self.files.contains_key(&name) {
            return;
        }

        let code_lines = file
            .src
            .as_ref()
            .map(|src| count_code_lines(src))
            .unwrap_or(0);

        self.files.entry(name).or_insert(FileInfo {
            hir_id,
            span,
            code_lines,
        });
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for info in self.files.values() {
            if info.code_lines > self.threshold {
                let over = info.code_lines - self.threshold;
                span_lint_hir_and_then(
                    cx,
                    EXCESSIVE_FILE_LENGTH,
                    info.hir_id,
                    info.span,
                    format!(
                        "file has {} code lines ({over} over the limit of {})",
                        info.code_lines, self.threshold,
                    ),
                    |diag| {
                        diag.help("consider splitting this file into smaller modules");
                    },
                );
            }
        }
    }
}
