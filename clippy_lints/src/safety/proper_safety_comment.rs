use rustc_ast::{
    AttrKind, AttrStyle, Attribute, Block, BlockCheckMode, ForeignMod, Impl, Item, ItemKind, Safety, UnsafeSource,
};
use rustc_lint::{EarlyContext, LintContext};
use rustc_middle::lint;
use rustc_span::{BytePos, SourceFileAndLine, Span};

use super::PROPER_SAFETY_COMMENT;

/// All safety comments have the format `// SAFETY_COMMENT_LABEL **comment**`
const SAFETY_COMMENT_LABEL: &str = "SAFETY:";

pub(super) fn check_attribute(cx: &EarlyContext<'_>, attr: &Attribute) {
    if lint::in_external_macro(cx.sess(), attr.span) {
        return;
    }

    let is_critical = match &attr.kind {
        AttrKind::Normal(p) => match p.item.unsafety {
            Safety::Unsafe(_) => true,
            Safety::Safe(_) | Safety::Default => false,
        },
        AttrKind::DocComment(_, _) => false,
    };

    if is_critical {
        // check for procedural macro
        let expected_tokens = match attr.style {
            AttrStyle::Outer => &["#", "", "[", "unsafe", "("],
            AttrStyle::Inner => &["#", "!", "[", "unsafe", "("],
        };
        if !span_starts_with(cx, attr.span, expected_tokens) {
            return;
        }

        if span_has_safety_comment(cx, attr.span).is_none() {
            clippy_utils::diagnostics::span_lint(
                cx,
                PROPER_SAFETY_COMMENT,
                attr.span,
                "missing safety comment on critical attribute",
            );
        }
    } else {
        // TODO what about `#[derive(..)]`?
        // // check for procedural macro
        // if !span_starts_with(cx, attr.span, &["#", "["]) {
        //     return;
        // }

        if let Some(span) = span_has_safety_comment(cx, attr.span) {
            clippy_utils::diagnostics::span_lint(
                cx,
                PROPER_SAFETY_COMMENT,
                span,
                "unnecessary safety comment on attribute",
            );
        }
    }
}

pub(super) fn check_block(cx: &EarlyContext<'_>, block: &Block) {
    if lint::in_external_macro(cx.sess(), block.span) {
        return;
    }

    let (is_unsafe, is_critical) = match block.rules {
        BlockCheckMode::Default => (false, false),
        BlockCheckMode::Unsafe(UnsafeSource::UserProvided) => (true, true),
        BlockCheckMode::Unsafe(UnsafeSource::CompilerGenerated) => (true, false),
    };

    match block_contains_safety_comment(cx, block.span, is_unsafe) {
        Ok(Some(span)) => {
            if !(is_critical || is_unsafe) {
                clippy_utils::diagnostics::span_lint(
                    cx,
                    PROPER_SAFETY_COMMENT,
                    span,
                    "unnecessary safety comment inside block",
                );
            }
        },
        Ok(None) => {
            if is_critical && is_unsafe {
                clippy_utils::diagnostics::span_lint(
                    cx,
                    PROPER_SAFETY_COMMENT,
                    block.span,
                    "missing safety comment inside unsafe block",
                );
            }
        },
        Err(()) => {},
    }
}

pub(super) fn check_item(cx: &EarlyContext<'_>, item: &Item) {
    if lint::in_external_macro(cx.sess(), item.span) {
        return;
    }

    match &item.kind {
        ItemKind::ForeignMod(foreign_mod) => check_foreign_mod(cx, foreign_mod, item.span),
        ItemKind::Impl(impl_block) => check_impl(cx, impl_block, item.span),
        _ => (),
    }
}

fn check_foreign_mod(cx: &EarlyContext<'_>, foreign_mod: &ForeignMod, span: Span) {
    // check for procedural macro
    if !span_starts_with(cx, span, &["unsafe", "extern"]) {
        //TODO remove this `if`-statement once `unsafe extern` is mandatory
        if !span_starts_with(cx, span, &["extern"]) {
            return;
        }
    }

    if span_has_safety_comment(cx, span).is_none() {
        clippy_utils::diagnostics::span_lint(
            cx,
            PROPER_SAFETY_COMMENT,
            span,
            "missing safety comment on `unsafe extern`-block",
        );
    }

    for foreign_mod_item in &foreign_mod.items {
        if span_has_safety_comment(cx, foreign_mod_item.span).is_none() {
            clippy_utils::diagnostics::span_lint(
                cx,
                PROPER_SAFETY_COMMENT,
                foreign_mod_item.span,
                "missing safety comment on item in `unsafe extern`-block",
            );
        }
    }
}

fn check_impl(cx: &EarlyContext<'_>, impl_block: &Impl, span: Span) {
    match impl_block.safety {
        Safety::Unsafe(_) => {
            // check for procedural macro
            if !span_starts_with(cx, span, &["unsafe", "impl"]) {
                return;
            }

            if span_has_safety_comment(cx, span).is_none() {
                clippy_utils::diagnostics::span_lint(
                    cx,
                    PROPER_SAFETY_COMMENT,
                    span,
                    "missing safety comment on unsafe impl",
                );
            }
        },
        Safety::Safe(_) => {},
        Safety::Default => {
            // check for procedural macro
            if !span_starts_with(cx, span, &["impl"]) {
                return;
            }

            if let Some(span) = span_has_safety_comment(cx, span) {
                clippy_utils::diagnostics::span_lint(
                    cx,
                    PROPER_SAFETY_COMMENT,
                    span,
                    "unnecessary safety comment on impl",
                );
            }
        },
    }
}

fn block_contains_safety_comment(cx: &impl LintContext, span: Span, is_unsafe: bool) -> Result<Option<Span>, ()> {
    let source_map = cx.sess().source_map();

    let snippet = source_map.span_to_snippet(span).map_err(|_| ())?;

    let trimmed_snippet = snippet
        .trim_start()
        .strip_prefix(if is_unsafe { "unsafe" } else { "" })
        .ok_or(())?
        .trim_start()
        .strip_prefix("{")
        .ok_or(())?
        .trim_start();

    if identify_text_type(trimmed_snippet) != TextType::SafetyComment {
        return Ok(None);
    }

    let safety_comment_start =
        span.lo() + BytePos(u32::try_from(snippet.len() - trimmed_snippet.len()).map_err(|_| ())?);

    let SourceFileAndLine {
        sf: safety_comment_source_file,
        line: safety_comment_start_line,
    } = source_map.lookup_line(safety_comment_start).map_err(|_| ())?;

    let mut safety_comment_end_line = safety_comment_start_line;
    while let TextType::Comment | TextType::Empty = identify_text_type(
        &safety_comment_source_file
            .get_line(safety_comment_end_line + 1)
            .ok_or(())?,
    ) {
        safety_comment_end_line += 1;
    }
    let safety_comment_end_line = safety_comment_end_line;

    let safety_comment = span
        .with_lo(safety_comment_start)
        .with_hi(safety_comment_source_file.line_bounds(safety_comment_end_line).end);

    Ok(Some(safety_comment))
}

#[must_use]
fn span_has_safety_comment(cx: &impl LintContext, span: Span) -> Option<Span> {
    let source_map = cx.sess().source_map();

    let SourceFileAndLine {
        sf: unsafe_source_file,
        line: unsafe_line_number,
    } = source_map.lookup_line(span.lo()).ok()?;

    for line_number in (0..unsafe_line_number).rev() {
        match identify_text_type(&unsafe_source_file.get_line(line_number)?) {
            TextType::SafetyComment => {
                let safety_comment = span
                    .with_lo(unsafe_source_file.line_bounds(line_number).start)
                    .with_hi(unsafe_source_file.line_bounds(unsafe_line_number - 1).end);

                return Some(safety_comment);
            },
            TextType::Comment | TextType::Empty => continue,
            TextType::NoComment | TextType::DocComment => break,
        }
    }

    None
}

#[must_use]
fn span_starts_with(cx: &impl LintContext, span: Span, tokens: &[&str]) -> bool {
    cx.sess()
        .source_map()
        .span_to_source(span, |src, start, end| {
            if let Some(snippet) = src.get(start..end) {
                let mut remaining_snippet = snippet;
                for &token in tokens {
                    if let Some(s) = remaining_snippet.strip_prefix(token) {
                        remaining_snippet = s.trim_start();
                    } else {
                        return Ok(false);
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .unwrap_or(false)
}

#[derive(Debug, PartialEq)]
enum TextType {
    SafetyComment,
    Comment,
    DocComment,
    NoComment,
    Empty,
}

#[must_use]
fn identify_text_type(text: &str) -> TextType {
    let text_trimmed = text.trim_start();

    if text_trimmed.starts_with("///") {
        TextType::DocComment
    } else if text_trimmed.starts_with("//") {
        let comment = text_trimmed.strip_prefix("//").unwrap().trim_start();

        if comment.starts_with(SAFETY_COMMENT_LABEL) {
            TextType::SafetyComment
        } else {
            TextType::Comment
        }
    } else if !text_trimmed.is_empty() {
        TextType::NoComment
    } else {
        TextType::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_text_type() {
        assert_eq!(
            identify_text_type(&format!("//{SAFETY_COMMENT_LABEL}")),
            TextType::SafetyComment
        );
        assert_eq!(
            identify_text_type(&format!(" // {SAFETY_COMMENT_LABEL} ")),
            TextType::SafetyComment
        );
        assert_eq!(
            identify_text_type(&format!("  //  {SAFETY_COMMENT_LABEL}  ")),
            TextType::SafetyComment
        );
        assert_eq!(identify_text_type("//"), TextType::Comment);
        assert_eq!(identify_text_type(" // "), TextType::Comment);
        assert_eq!(identify_text_type("  //  "), TextType::Comment);
        assert_eq!(
            identify_text_type(&format!("///{SAFETY_COMMENT_LABEL}")),
            TextType::DocComment
        );
        assert_eq!(
            identify_text_type(&format!(" /// {SAFETY_COMMENT_LABEL} ")),
            TextType::DocComment
        );
        assert_eq!(
            identify_text_type(&format!("  ///  {SAFETY_COMMENT_LABEL}  ")),
            TextType::DocComment
        );
        assert_eq!(
            identify_text_type(&format!("/{SAFETY_COMMENT_LABEL}")),
            TextType::NoComment
        );
        assert_eq!(
            identify_text_type(&format!(" / {SAFETY_COMMENT_LABEL} ")),
            TextType::NoComment
        );
        assert_eq!(
            identify_text_type(&format!("  /  {SAFETY_COMMENT_LABEL}  ")),
            TextType::NoComment
        );
        assert_eq!(identify_text_type(SAFETY_COMMENT_LABEL), TextType::NoComment);
        assert_eq!(identify_text_type(""), TextType::Empty);
        assert_eq!(identify_text_type("    \n    \n\n    "), TextType::Empty);
    }
}
