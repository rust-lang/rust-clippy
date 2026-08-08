use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::numeric_literal::{FloatStr, IntStr, Radix, reformat_whole_opt_fract};
use clippy_utils::source::SpanExt;
use core::ptr;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, Lint, LintContext};
use rustc_span::Span;

use super::{INCONSISTENT_DIGIT_GROUPING, LARGE_DIGIT_GROUPS, UNREADABLE_LITERAL, UNUSUAL_BYTE_GROUPINGS};

struct LintInfo {
    lint: &'static Lint,
    msg: &'static str,
    sugg_msg: &'static str,
}
impl PartialEq for LintInfo {
    fn eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self, other)
    }
}
static INCONSISTENT_DIGIT_GROUPING_INFO: LintInfo = LintInfo {
    lint: INCONSISTENT_DIGIT_GROUPING,
    msg: "digits in groups of unequal sizes",
    sugg_msg: "group the digits with a consistent size",
};
static LARGE_DIGIT_GROUPS_INFO: LintInfo = LintInfo {
    lint: LARGE_DIGIT_GROUPS,
    msg: "digits separated into large groups",
    sugg_msg: "split the digits into smaller groups",
};
static UNREADABLE_LITERAL_INFO: LintInfo = LintInfo {
    lint: UNREADABLE_LITERAL,
    msg: "long literal lacking separators",
    sugg_msg: "separate the digits",
};
static UNUSUAL_BYTE_GROUPINGS_INFO: LintInfo = LintInfo {
    lint: UNUSUAL_BYTE_GROUPINGS,
    msg: "digits in groups of unequal sizes",
    sugg_msg: "group the digits with a consistent size",
};

pub(crate) fn check_int(cx: &EarlyContext<'_>, num: IntStr<'_>, sp: Span) {
    let l = 'check: {
        let mut groups = num.text.split('_').map(str::len);
        let first = groups.next().unwrap();
        let Some(second) = groups.next() else {
            if num.text.len() > 5 {
                break 'check &UNREADABLE_LITERAL_INFO;
            }
            return;
        };

        let inconsistent_groups = if first > second {
            // Allow uuid formatted numbers
            !(num.radix == Radix::Hex
                && first == 8
                && second == 4
                && groups.next() == Some(4)
                && groups.next() == Some(4)
                && groups.next() == Some(12)
                && groups.next().is_none())
        } else {
            groups.any(|x| x != second)
        };

        if inconsistent_groups {
            break 'check if let Radix::Dec = num.radix {
                &INCONSISTENT_DIGIT_GROUPING_INFO
            } else {
                &UNUSUAL_BYTE_GROUPINGS_INFO
            };
        } else if second > 4 && num.radix == Radix::Dec {
            break 'check &LARGE_DIGIT_GROUPS_INFO;
        };
        return;
    };

    let sp_data = sp.data();
    if !sp_data.ctxt.in_external_macro(cx.sess().source_map()) && sp_data.check_text(cx, |src| num.eq_str(src)) {
        span_lint_and_then(cx, l.lint, sp, l.msg, |diag| {
            diag.span_suggestion_verbose(
                num.trim_sp_to_digits(&sp_data),
                l.sugg_msg,
                num.reformat_digits(),
                Applicability::MachineApplicable,
            );
        });
    }
}

pub(super) fn check_float(cx: &EarlyContext<'_>, num: &FloatStr<'_>, lint_unreadable_fract: bool, sp: Span) {
    let (whole, fract) = num.whole_fract_digits_str();
    let (whole_lint, whole_group_size) = 'check_whole: {
        let mut groups = whole.split('_').map(str::len);
        let first = groups.next().unwrap();
        let Some(second) = groups.next() else {
            break 'check_whole ((first > 5).then_some(&UNREADABLE_LITERAL_INFO), None);
        };
        let l = if first > second || groups.any(|x| x != second) {
            Some(&INCONSISTENT_DIGIT_GROUPING_INFO)
        } else if second > 4 {
            Some(&LARGE_DIGIT_GROUPS_INFO)
        } else {
            None
        };
        (l, Some(second))
    };

    let (fract_lint, fract_group_size) = 'check_fract: {
        let Some(fract) = fract else {
            break 'check_fract (None, None);
        };
        let mut groups = fract.rsplit('_').map(str::len);
        let first = groups.next().unwrap();
        let Some(second) = groups.next() else {
            break 'check_fract (
                (lint_unreadable_fract && first > 5).then_some(&UNREADABLE_LITERAL_INFO),
                None,
            );
        };
        let l = if first > second || groups.any(|x| x != second) {
            Some(&INCONSISTENT_DIGIT_GROUPING_INFO)
        } else if second > 4 {
            Some(&LARGE_DIGIT_GROUPS_INFO)
        } else {
            None
        };
        (l, Some(second))
    };

    let whole_fract_inconsistent = whole_group_size.zip(fract_group_size).is_some_and(|(x, y)| x != y);
    if (whole_lint.is_some() || fract_lint.is_some() || whole_fract_inconsistent)
        && let sp_data = sp.data()
        && sp_data.check_text(cx, |src| num.eq_str(src))
    {
        let l = if whole_fract_inconsistent
            || whole_lint == Some(&INCONSISTENT_DIGIT_GROUPING_INFO)
            || fract_lint == Some(&INCONSISTENT_DIGIT_GROUPING_INFO)
        {
            &INCONSISTENT_DIGIT_GROUPING_INFO
        } else if whole_lint == Some(&LARGE_DIGIT_GROUPS_INFO) || fract_lint == Some(&LARGE_DIGIT_GROUPS_INFO) {
            &LARGE_DIGIT_GROUPS_INFO
        } else {
            &UNREADABLE_LITERAL_INFO
        };
        span_lint_and_then(cx, l.lint, sp, l.msg, |diag| {
            diag.span_suggestion_verbose(
                num.strip_exp_from_sp(&sp_data),
                l.sugg_msg,
                reformat_whole_opt_fract(whole, fract),
                Applicability::MachineApplicable,
            );
        });
    }
}
