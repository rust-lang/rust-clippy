use std::iter::Peekable;
use std::str::Chars;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::format_arg_removal_span;
use clippy_utils::source::SpanExt as _;
use clippy_utils::sym;
use rustc_ast::token::LitKind;
use rustc_ast::{
    FormatArgPosition, FormatArgPositionKind, FormatArgs, FormatArgsPiece, FormatCount, FormatOptions,
    FormatPlaceholder, FormatTrait,
};
use rustc_errors::Applicability;
use rustc_lint::LateContext;
use rustc_span::Span;

use super::{PRINT_LITERAL, WRITE_LITERAL};

pub(super) fn check(cx: &LateContext<'_>, format_args: &FormatArgs, name: &str) {
    let arg_index = |argument: &FormatArgPosition| argument.index.unwrap_or_else(|pos| pos);

    let lint_name = if name.starts_with("write") {
        WRITE_LITERAL
    } else {
        PRINT_LITERAL
    };

    let mut counts = vec![0u32; format_args.arguments.all_args().len()];
    for piece in &format_args.template {
        if let FormatArgsPiece::Placeholder(placeholder) = piece {
            counts[arg_index(&placeholder.argument)] += 1;
        }
    }

    let mut suggestion: Vec<(Span, String)> = vec![];
    // holds index of replaced positional arguments; used to decrement the index of the remaining
    // positional arguments.
    let mut replaced_position: Vec<usize> = vec![];
    let mut sug_span: Option<Span> = None;

    for piece in &format_args.template {
        if let FormatArgsPiece::Placeholder(FormatPlaceholder {
            argument,
            span: Some(placeholder_span),
            format_trait: FormatTrait::Display,
            format_options,
        }) = piece
            && *format_options == FormatOptions::default()
            && let index = arg_index(argument)
            && counts[index] == 1
            && let Some(arg) = format_args.arguments.by_index(index)
            && let rustc_ast::ExprKind::Lit(lit) = &arg.expr.kind
            && !arg.expr.span.from_expansion()
            && let Some(value_string) = arg.expr.span.get_text(cx)
        {
            let (replacement, replace_raw) = match lit.kind {
                LitKind::Str | LitKind::StrRaw(_) => match extract_str_literal(&value_string) {
                    Some(extracted) => extracted,
                    None => return,
                },
                LitKind::Char => (
                    match lit.symbol {
                        sym::DOUBLE_QUOTE => "\\\"",
                        sym::BACKSLASH_SINGLE_QUOTE => "'",
                        _ => match value_string.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                            Some(stripped) => stripped,
                            None => return,
                        },
                    }
                    .to_string(),
                    false,
                ),
                LitKind::Bool => (lit.symbol.to_string(), false),
                _ => continue,
            };

            let Some(format_string_snippet) = format_args.span.get_text(cx) else {
                continue;
            };
            let format_string_is_raw = format_string_snippet.starts_with('r');

            let replacement = match (format_string_is_raw, replace_raw) {
                (false, false) => Some(replacement),
                (false, true) => Some(replacement.replace('\\', "\\\\").replace('"', "\\\"")),
                (true, false) => match conservative_unescape(&replacement) {
                    Ok(unescaped) => Some(unescaped),
                    Err(UnescapeErr::Lint) => None,
                    Err(UnescapeErr::Ignore) => continue,
                },
                (true, true) => {
                    if replacement.contains(['#', '"']) {
                        None
                    } else {
                        Some(replacement)
                    }
                },
            };

            sug_span = Some(sug_span.unwrap_or(arg.expr.span).to(arg.expr.span));

            if let Some((_, index)) = format_arg_piece_span(piece) {
                replaced_position.push(index);
            }

            if let Some(replacement) = replacement
                // `format!("{}", "a")`, `format!("{named}", named = "b")
                //              ~~~~~                      ~~~~~~~~~~~~~
                && let Some(removal_span) = format_arg_removal_span(format_args, index)
            {
                let replacement = escape_braces(&replacement, !format_string_is_raw && !replace_raw);
                suggestion.push((*placeholder_span, replacement));
                suggestion.push((removal_span, String::new()));
            }
        }
    }

    // Decrement the index of the remaining by the number of replaced positional arguments
    if !suggestion.is_empty() {
        for piece in &format_args.template {
            relocalize_format_args_indexes(piece, &mut suggestion, &replaced_position);
        }
    }

    if let Some(span) = sug_span {
        span_lint_and_then(cx, lint_name, span, "literal with an empty format string", |diag| {
            if !suggestion.is_empty() {
                diag.multipart_suggestion("try", suggestion, Applicability::MachineApplicable);
            }
        });
    }
}

/// Extract Span and its index from the given `piece`
fn format_arg_piece_span(piece: &FormatArgsPiece) -> Option<(Span, usize)> {
    match piece {
        FormatArgsPiece::Placeholder(FormatPlaceholder {
            argument: FormatArgPosition { index: Ok(index), .. },
            span: Some(span),
            ..
        }) => Some((*span, *index)),
        _ => None,
    }
}

/// Relocalizes the indexes of positional arguments in the format string
fn relocalize_format_args_indexes(
    piece: &FormatArgsPiece,
    suggestion: &mut Vec<(Span, String)>,
    replaced_position: &[usize],
) {
    if let FormatArgsPiece::Placeholder(FormatPlaceholder {
        argument:
            FormatArgPosition {
                index: Ok(index),
                // Only consider positional arguments
                kind: FormatArgPositionKind::Number,
                span: Some(span),
            },
        format_options,
        ..
    }) = piece
    {
        if suggestion.iter().any(|(s, _)| s.overlaps(*span)) {
            // If the span is already in the suggestion, we don't need to process it again
            return;
        }

        // lambda to get the decremented index based on the replaced positions
        let decremented_index = |index: usize| -> usize {
            let decrement = replaced_position.iter().filter(|&&i| i < index).count();
            index - decrement
        };

        suggestion.push((*span, decremented_index(*index).to_string()));

        // If there are format options, we need to handle them as well
        if *format_options != FormatOptions::default() {
            // lambda to process width and precision format counts and add them to the suggestion
            let mut process_format_count = |count: &Option<FormatCount>, formatter: &dyn Fn(usize) -> String| {
                if let Some(FormatCount::Argument(FormatArgPosition {
                    index: Ok(format_arg_index),
                    kind: FormatArgPositionKind::Number,
                    span: Some(format_arg_span),
                })) = count
                {
                    suggestion.push((*format_arg_span, formatter(decremented_index(*format_arg_index))));
                }
            };

            process_format_count(&format_options.width, &|index: usize| format!("{index}$"));
            process_format_count(&format_options.precision, &|index: usize| format!(".{index}$"));
        }
    }
}

/// Removes the raw marker, `#`s and quotes from a str, and returns if the literal is raw
///
/// `r#"a"#` -> (`a`, true)
///
/// `"b"` -> (`b`, false)
fn extract_str_literal(literal: &str) -> Option<(String, bool)> {
    let (literal, raw) = match literal.strip_prefix('r') {
        Some(stripped) => (stripped.trim_matches('#'), true),
        None => (literal, false),
    };

    Some((literal.strip_prefix('"')?.strip_suffix('"')?.to_string(), raw))
}

enum UnescapeErr {
    /// Should still be linted, can be manually resolved by author, e.g.
    ///
    /// ```ignore
    /// print!(r"{}", '"');
    /// ```
    Lint,
    /// Should not be linted, e.g.
    ///
    /// ```ignore
    /// print!(r"{}", '\r');
    /// ```
    Ignore,
}

/// Unescape a normal string into a raw string
fn conservative_unescape(literal: &str) -> Result<String, UnescapeErr> {
    let mut unescaped = String::with_capacity(literal.len());
    let mut chars = literal.chars();
    let mut err = false;

    while let Some(ch) = chars.next() {
        match ch {
            '#' => err = true,
            '\\' => match chars.next() {
                Some('\\') => unescaped.push('\\'),
                Some('"') => err = true,
                _ => return Err(UnescapeErr::Ignore),
            },
            _ => unescaped.push(ch),
        }
    }

    if err { Err(UnescapeErr::Lint) } else { Ok(unescaped) }
}

/// Doubles up every brace so that a string literal can be safely inlined into a (non-raw) format
/// string. This covers literal braces (`{`, `}`) as well as braces produced by escape sequences
/// (`\x7b`, `\u{7d}`), because the format string parser sees them only after unescaping. Escape
/// sequences that do not evaluate to a brace (e.g. `\u{ab123}`) and the braces that are part of the
/// `\u{…}` escape syntax are left untouched.
///
/// If `preserve_unicode_escapes` is `false` (the literal ends up in a raw format string) escape
/// sequences do not exist, so only literal braces are doubled up.
fn escape_braces(literal: &str, preserve_unicode_escapes: bool) -> String {
    let mut escaped = String::with_capacity(literal.len());
    let mut chars = literal.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // A brace in the value: double it up.
            '{' | '}' => {
                escaped.push(ch);
                escaped.push(ch);
            },
            // An escape sequence may hide a brace (`\x7b`) or contain braces that belong to the
            // escape syntax rather than the value (`\u{…}`), so it needs special handling.
            '\\' if preserve_unicode_escapes => escape_backslash(&mut escaped, &mut chars),
            _ => escaped.push(ch),
        }
    }

    escaped
}

/// Handles a `\`-escape for [`escape_braces`] when unicode escapes are preserved. The leading `\`
/// has already been matched by the caller; this consumes the rest of the escape from `chars` and
/// appends it. `\x7b`/`\x7d` and `\u{7b}`/`\u{7d}` decode to braces, so the whole escape is doubled
/// up to survive the format-string parser; every other escape (and any non-brace `\u{…}`) is copied
/// verbatim so it is not re-scanned as the start of another escape or as a brace.
fn escape_backslash(escaped: &mut String, chars: &mut Peekable<Chars<'_>>) {
    escaped.push('\\');
    match chars.peek().copied() {
        // `\x7b` / `\x7d`: a brace written as a hex escape.
        Some('x') => {
            escaped.push('x');
            chars.next();
            let mut digits = String::new();
            while digits.len() < 2 {
                match chars.peek() {
                    Some(&c) if c.is_ascii_hexdigit() => {
                        digits.push(c);
                        escaped.push(c);
                        chars.next();
                    },
                    _ => break,
                }
            }
            if matches!(u32::from_str_radix(&digits, 16), Ok(0x7B | 0x7D)) {
                escaped.push('\\');
                escaped.push('x');
                escaped.push_str(&digits);
            }
        },
        // `\u{7b}` / `\u{7d}`: a brace written as a unicode escape. Other unicode escapes
        // (`\u{ab123}`) are copied verbatim, braces and all.
        Some('u') => {
            escaped.push('u');
            chars.next();
            if chars.peek() == Some(&'{') {
                let mut sequence = String::from("\\u");
                let mut value = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    escaped.push(c);
                    sequence.push(c);
                    match c {
                        '}' => {
                            closed = true;
                            break;
                        },
                        '_' | '{' => {},
                        _ => value.push(c),
                    }
                }
                if closed && matches!(u32::from_str_radix(&value, 16), Ok(0x7B | 0x7D)) {
                    escaped.push_str(&sequence);
                }
            }
        },
        // Any other escape (`\\`, `\n`, `\"`, …): copy the escaped character verbatim.
        Some(c) => {
            escaped.push(c);
            chars.next();
        },
        None => {},
    }
}
