use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::format_arg_removal_span;
use clippy_utils::source::SpanRangeExt;
use clippy_utils::str_utils::{InlineEscapeError, inline_literal_in_format_string};
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
            && let Some(value_string) = arg.expr.span.get_source_text(cx)
        {
            let Some(format_string_snippet) = format_args.span.get_source_text(cx) else {
                continue;
            };
            let format_string_is_raw = format_string_snippet.starts_with('r');

            let replacement = match inline_literal_in_format_string(lit, &value_string, format_string_is_raw) {
                Ok(inlined) => Some(inlined),
                Err(InlineEscapeError::FormatArgsUnescapable) => return,
                Err(InlineEscapeError::CurrentArgUnescapable { fixable_by_user: true }) => None,
                _ => continue,
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
