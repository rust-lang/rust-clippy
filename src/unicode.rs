extern crate unicode_normalization;

use std::fmt::Write;
use rustc::lint::*;
use syntax::ast::*;
use syntax::codemap::{Pos, BytePos, Span};
use self::unicode_normalization::char::canonical_combining_class;
use self::unicode_normalization::UnicodeNormalization;

use utils::span_lint;

declare_lint!{ pub ZERO_WIDTH_SPACE, Deny,
               "using a zero-width space in a string literal, which is confusing" }
declare_lint!{ pub NON_ASCII_LITERAL, Allow,
               "using any literal non-ASCII chars in a string literal; suggests \
                using the \\u escape instead" }
declare_lint!{ pub UNICODE_NOT_NFC, Allow,
               "using a unicode literal not in NFC normal form (see \
               http://www.unicode.org/reports/tr15/ for further information)" }

#[derive(Copy, Clone)]
pub struct Unicode;

impl LintPass for Unicode {
    fn get_lints(&self) -> LintArray {
        lint_array!(ZERO_WIDTH_SPACE, NON_ASCII_LITERAL, UNICODE_NOT_NFC)
    }

    fn check_expr(&mut self, cx: &Context, expr: &Expr) {
        if let ExprLit(ref lit) = expr.node {
            if let LitStr(ref string, _) = lit.node {
                check_str(cx, string, lit.span)
            }
        }
    }
}

fn pos(base: BytePos, i: usize) -> BytePos {
    if i == 0 { base } else { base + Pos::from_usize(i + 1) }
}

#[allow(cast_possible_truncation)]
fn str_pos_lint(cx: &Context, lint: &'static Lint, span: Span, index: usize,
        end_index: Option<usize>, msg: &str) {

    span_lint(cx, lint,
        Span {
            lo: pos(span.lo, index),
            hi: end_index.map_or(span.hi, |i| pos(span.lo, i)),
            expn_id: span.expn_id,
        },
        msg);
}


fn push_start(from: &mut Option<usize>, til: Option<usize>,
        v: &mut Vec<(usize, Option<usize>)>) {
    if let Some(s) = from.take() {
        v.push((s, til));
    }
}

fn push_last_and_report<F>(cx: &Context, string: &str, span: Span,
        mut from: Option<usize>, mut ranges: Vec<(usize, Option<usize>)>,
        lint: &'static Lint, prefix: &str, multi_fun: F)
where F: Fn(&str) -> String, {
    push_start(&mut from, None, &mut ranges);
    match ranges.len() {
        0 => (),
        1 => {
            let range = ranges[0];
            str_pos_lint(cx, lint, span, range.0, range.1, &format!(
                "{} range detected. Consider using `{}`",
                prefix,
                &if let Some(u) = range.1 {
                    multi_fun(&string[range.0 .. u])
                } else {
                    multi_fun(&string[range.0 ..])
                }
            ));
        },
        x => {
            let mut repls = String::new();
            for (from, until) in ranges {
                if let Some(u) = until {
                    write!(&mut repls, "\n{}..{} => {}",
                        from, u, &multi_fun(&string[from..u])).expect("");
                } else {
                    write!(&mut repls, "\n{}.. => {}",
                        from, &multi_fun(&string[from..])).expect("");
                }
            }
            span_lint(cx, lint, span, &format!(
                "{} {} ranges detected. Consider the following replacements:{}",
                x, prefix, &repls));
        }
    }
}

fn check_str(cx: &Context, string: &str, span: Span) {
    let mut zero_width_ranges = vec![];
    let mut non_ascii_ranges = vec![];
    let mut non_nfc_ranges = vec![];
    let mut zero_width_start = None;
    let mut non_ascii_start = None;
    let mut non_nfc_start = None;
    let mut last_base_char = None;
    for (i, c) in string.char_indices() {
        if c == '\u{200B}' {
            if zero_width_start.is_none() {
                zero_width_start = Some(i);
            }
        } else {
            push_start(&mut zero_width_start, Some(i), &mut zero_width_ranges);
        }
        if c as u32 > 0x7F {
            if non_ascii_start.is_none() {
                non_ascii_start = Some(i);
            }
        } else {
            push_start(&mut non_ascii_start, Some(i), &mut non_ascii_ranges);
        }
        if canonical_combining_class(c) == 0 { // not a combining char
            if let Some(l) = last_base_char {
                let seq = &string[l..i];
                if seq.nfc().zip(seq.chars()).any(|(a, b)| a != b) {
                    if non_nfc_start.is_none() {
                        non_nfc_start = last_base_char;
                    }
                } else {
                    if let Some(nns) = non_nfc_start.take() {
                        non_nfc_ranges.push((nns, Some(i)));
                    }
                }
            }
            last_base_char = Some(i);
        }
    }
    push_last_and_report(cx, string, span, zero_width_start, zero_width_ranges,
        ZERO_WIDTH_SPACE, "zero-width space", zero_width_replacement);
    push_last_and_report(cx, string, span, non_ascii_start, non_ascii_ranges,
        NON_ASCII_LITERAL, "non-ascii literal", non_ascii_replacement);
    if cx.current_level(NON_ASCII_LITERAL) == Level::Allow {
        push_last_and_report(cx, string, span, non_nfc_start, non_nfc_ranges,
            UNICODE_NOT_NFC, "non-NFC unicode", non_nfc_replacement);
    } else {
        push_last_and_report(cx, string, span, non_nfc_start, non_nfc_ranges,
            UNICODE_NOT_NFC, "non-NFC unicode", non_nfc_ascii_replacement);
    }
}

fn zero_width_replacement(string: &str) -> String {
    string.chars().map(|_| "\\u{200B}").collect()
}

fn non_ascii_replacement(string: &str) -> String {
    string.chars().flat_map(char::escape_unicode).collect()
}

fn non_nfc_replacement(string: &str) -> String {
    string.nfc().collect()
}

fn non_nfc_ascii_replacement(string: &str) -> String {
    string.nfc().flat_map(char::escape_unicode).collect()
}
