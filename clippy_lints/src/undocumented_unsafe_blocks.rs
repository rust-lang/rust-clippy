use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast::ast::{BlockCheckMode, ExprKind, UnsafeSource};
use rustc_data_structures::fx::FxHashMap;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{source_map::SourceMap, BytePos, FileName, Span};
use std::convert::TryInto;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `unsafe` blocks without a `// SAFETY` comment
    /// explaining why the unsafe operations performed inside
    /// the block are safe.
    ///
    /// ### Why is this bad?
    /// Undocumented unsafe blocks are hard to check and maintain.
    /// On the other hand, writing safety comments helps uncovering
    /// unsoundness holes and bugs.
    ///
    /// ### Example
    /// ```rust
    /// # use std::ptr::NonNull;
    /// # let a = &mut 42;
    ///
    /// let ptr = unsafe { NonNull::new_unchecked(a) };
    /// ```
    /// You should explain why calling `NonNull::new_unchecked` is safe:
    /// ```rust
    /// # use std::ptr::NonNull;
    /// # let a = &mut 42;
    ///
    /// // SAFETY: references are guaranteed to be non-null.
    /// let ptr = unsafe { NonNull::new_unchecked(a) };
    /// ```
    pub UNDOCUMENTED_UNSAFE_BLOCKS,
    restriction,
    "unsafe blocks without safety comments"
}

struct SafetyComment {
    hi: BytePos,
    hi_line: usize,
}

#[derive(Default)]
pub struct UndocumentedUnsafeBlocks {
    safety_comments: FxHashMap<FileName, Option<Vec<SafetyComment>>>,
}

impl UndocumentedUnsafeBlocks {
    fn safety_comments(&mut self, sm: &SourceMap, span: Span) -> Option<&mut Vec<SafetyComment>> {
        let file = sm.span_to_filename(span);
        self.safety_comments
            .entry(file.clone())
            .or_insert_with(|| Self::gather_safety_comments(sm, &file))
            .as_mut()
    }

    // Inspired by `rustc_ast::utils::comments::gather_comments`.
    fn gather_safety_comments(sm: &SourceMap, file: &FileName) -> Option<Vec<SafetyComment>> {
        let source_file = sm.get_source_file(file)?;
        let src = source_file.src.as_deref()?;
        let start_bpos = source_file.start_pos;
        let mut comments = Vec::new();

        let mut inside_comment = false;
        let mut pos = rustc_lexer::strip_shebang(src).unwrap_or(0);
        for token in rustc_lexer::tokenize(&src[pos..]) {
            match token.kind {
                rustc_lexer::TokenKind::LineComment { doc_style: None }
                | rustc_lexer::TokenKind::BlockComment {
                    doc_style: None,
                    terminated: true,
                } if src[pos + 2..pos + token.len].trim_start().starts_with("SAFETY") => {
                    inside_comment = true;
                },
                rustc_lexer::TokenKind::LineComment { doc_style: None }
                | rustc_lexer::TokenKind::BlockComment { doc_style: None, .. }
                | rustc_lexer::TokenKind::Whitespace => {},
                _ => {
                    if inside_comment {
                        let hi = start_bpos + BytePos(pos.try_into().unwrap());
                        comments.push(SafetyComment {
                            hi,
                            hi_line: source_file.lookup_file_pos_with_col_display(hi).0,
                        });
                        inside_comment = false;
                    }
                },
            }
            pos += token.len;
        }
        Some(comments)
    }
}

impl_lint_pass!(UndocumentedUnsafeBlocks => [UNDOCUMENTED_UNSAFE_BLOCKS]);

impl EarlyLintPass for UndocumentedUnsafeBlocks {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &rustc_ast::Expr) {
        let (block, comments) = if_chain! {
            if let ExprKind::Block(ref block, _) = expr.kind;
            if let BlockCheckMode::Unsafe(UnsafeSource::UserProvided) = block.rules;
            if let Some(comments) = self.safety_comments(cx.sess.source_map(), block.span);
            then {
                (block, comments)
            }
            else {
                return;
            }
        };
        if_chain! {
            // Since we're consuming comments as we visit the AST, the comment
            // we're searching for is likely to be at the beginning of the vector,
            // so this is better than a binary search.
            if let Some((i, comment)) = comments.iter().enumerate().take_while(|(_, c)| c.hi <= block.span.lo()).last();
            let block_line = cx.sess.source_map().lookup_char_pos(block.span.lo()).line;
            if block_line == comment.hi_line + 1 || block_line == comment.hi_line;
            then {
                comments.remove(i);
            }
            else {
                span_lint(cx, UNDOCUMENTED_UNSAFE_BLOCKS, expr.span, "this `unsafe` block is missing a SAFETY comment");
            }
        }
    }
}
