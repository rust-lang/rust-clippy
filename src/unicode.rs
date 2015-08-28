extern crate unicode_normalization;

use rustc::lint::*;
use syntax::ast::*;
use syntax::codemap::{BytePos, Span};
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

fn check_str(cx: &Context, string: &str, span: Span) {
    let mut ustart = None;
    let mut last = None;
    for (i, c) in string.char_indices() {
        if c == '\u{200B}' {
            str_pos_lint(cx, ZERO_WIDTH_SPACE, span, i, Some(i),
                         "zero-width space detected. Consider using `\\u{200B}`");
        }
        if c as u32 > 0x7F {
            str_pos_lint(cx, NON_ASCII_LITERAL, span, i, Some(i), &format!(
                "literal non-ASCII character detected. Consider using `\\u{{{:X}}}`", c as u32));
        }
        if canonical_combining_class(c) == 0 { // not a combining char
            if let Some(l) = last {
                let seq = &string[l..i];
                if seq.nfc().zip(seq.chars()).any(|(a, b)| a != b) {
                    if ustart.is_none() { ustart = last; }
                } else {
                    if let Some(s) = ustart {
                        str_pos_lint(cx, UNICODE_NOT_NFC, span, s, Some(i),
                            &format!("non NFC-normal unicode sequence found. \
                                Consider using the normal form instead: '{}'", 
                                &string[s..i].nfc().collect::<String>()));
                    }
                    ustart = None;
                }
            }
            last = Some(i);
        }
    }    
    if let Some(s) = ustart {
        str_pos_lint(cx, UNICODE_NOT_NFC, span, s, None,
            &format!("non NFC-normal unicode sequence found. \
                Consider using the normal form instead: '{}'", 
                &string[s..].nfc().collect::<String>()));        
    }
}

#[allow(cast_possible_truncation)]
fn str_pos_lint(cx: &Context, lint: &'static Lint, span: Span, index: usize, 
        end_index: Option<usize>, msg: &str) {
    span_lint(cx, lint, Span { lo: span.lo + BytePos((1 + index) as u32),
                               hi: end_index.map_or(span.hi, 
                                    |i| span.lo + BytePos((1 + i) as u32)),
                               expn_id: span.expn_id }, msg);
}
